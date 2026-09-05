use super::*;

#[test]
fn nonindexed_owned_direct_parameters_clone_and_reject_mutation() {
    for vector in [false, true] {
        for exclusive in [false, true] {
            let (source, raw) = fixture(vector, exclusive, false, "direct");
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated direct parameter");
            let result = lower(pair_input(&syntax, &sources));
            if exclusive {
                let errors = result.expect_err("parameters are immutable");
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].code(), "ZRYNA-M3014");
                assert_eq!(
                    errors[0].message(),
                    "borrowed owner is immutable for mutation, unavailable, or partially moved"
                );
                assert_eq!(
                    errors[0].guidance(),
                    "borrow a complete initialized owner with mutable access for BorrowMut"
                );
                let start = u32::try_from(
                    source.find("borrowMut(incoming)").expect("borrow source") + "borrowMut(".len(),
                )
                .expect("offset");
                assert_eq!(
                    errors[0].primary_span().map(|span| (span.start(), span.end())),
                    Some((start, start + 8))
                );
                assert_eq!(
                    errors,
                    lower(pair_input(&syntax, &sources))
                        .expect_err("repeat direct parameter rejection")
                );
            } else {
                let program = result.unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let instructions =
                    function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                let cloned = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::GenericCloneBorrow)
                    .expect("parameter clone");
                let end = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .expect("end parameter borrow");
                assert!(cloned < end);
                assert!(
                    instructions[end + 1..]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::MoveFromPlace)
                );
            }
        }
    }
}

fn alias(f: &mut Builder, ty: &Ty, exclusive: bool, rhs: &str) {
    let local_start = f.source.len();
    let keyword_span = f.text("const");
    f.text(" ");
    let alias_name = f.name("loan");
    f.text(": ");
    let type_start = f.source.len();
    let borrow_keyword = f.text(if exclusive { "BorrowMut" } else { "Borrow" });
    let less_than_span = f.text("<");
    let argument = f.ty(ty);
    let greater_than_span = f.text(">");
    let type_syntax = u32::try_from(f.types.len()).expect("type");
    let kind = if exclusive {
        RawTypeSyntaxKind::BorrowMut {
            keyword_span: borrow_keyword,
            less_than_span,
            argument,
            greater_than_span,
        }
    } else {
        RawTypeSyntaxKind::Borrow {
            keyword_span: borrow_keyword,
            less_than_span,
            argument,
            greater_than_span,
        }
    };
    f.types.push(RawTypeSyntax { span: at(type_start, f.source.len()), kind });
    f.text(" ");
    let equals_span = f.text("=");
    f.text(" ");
    let borrow_start = f.source.len();
    let keyword = f.text(if exclusive { "borrowMut" } else { "borrow" });
    let open_paren_span = f.text("(");
    let value = f.reference(if matches!(rhs, "moved" | "direct") { "incoming" } else { "items" });
    let close_paren_span = f.text(")");
    let kind = if exclusive {
        RawExpressionKind::BorrowMut {
            keyword_span: keyword,
            open_paren_span,
            value,
            close_paren_span,
        }
    } else {
        RawExpressionKind::Borrow {
            keyword_span: keyword,
            open_paren_span,
            value,
            close_paren_span,
        }
    };
    let initializer = f.expression(borrow_start, kind);
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(local_start, f.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name: alias_name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
        },
    });
    f.text(" ");
}

fn action(f: &mut Builder, ty: &Ty, replace: bool, rhs: &str) {
    let action_start = f.source.len();
    let kind = if replace {
        let target = f.reference("loan");
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let value = if rhs == "wrong" {
            let start = f.source.len();
            f.text("0");
            f.expression(start, RawExpressionKind::I32Literal { spelling: "0".into() })
        } else {
            f.clone_value(|f| {
                f.reference(if matches!(rhs, "moved" | "direct") { "loan" } else { rhs })
            })
        };
        RawStatementKind::Assignment { target, equals_span, value, semicolon_span: f.text(";") }
    } else {
        let keyword_span = f.text("const");
        f.text(" ");
        let name = f.name("seen");
        f.text(": ");
        let type_syntax = f.ty(ty);
        f.text(" ");
        let equals_span = f.text("=");
        f.text(" ");
        let initializer = f.clone_value(|f| f.reference("loan"));
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span: f.text(";"),
        }
    };
    f.statements.push(RawStatementSyntax { span: at(action_start, f.source.len()), kind });
    f.text(" ");
}

#[test]
fn nonindexed_owned_roots_reject_invalid_replacements_deterministically() {
    for vector in [false, true] {
        for rhs in ["items", "wrong", "moved"] {
            let (source, raw) = fixture(vector, true, true, rhs);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated negative");
            let first = lower(pair_input(&syntax, &sources)).expect_err("invalid replacement");
            assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("repeat rejection"));
            let (code, message, guidance, spelling) = match rhs {
                "items" => (
                    "ZRYNA-M3014",
                    "owner access conflicts with an active borrow",
                    "end the conflicting borrow before accessing or consuming its owner",
                    "clone(items)",
                ),
                "wrong" => (
                    "ZRYNA-M3013",
                    "expression does not produce the exact required owned type",
                    "prepare a value matching the borrowed String or Vec referent type",
                    "0",
                ),
                "moved" => (
                    "ZRYNA-M3014",
                    "borrowed owner is immutable for mutation, unavailable, or partially moved",
                    "borrow a complete initialized owner with mutable access for BorrowMut",
                    "incoming",
                ),
                _ => unreachable!("negative case"),
            };
            assert_eq!(first.len(), 1);
            assert_eq!(
                (first[0].code(), first[0].message(), first[0].guidance()),
                (code, message, guidance)
            );
            let start =
                u32::try_from(source.rfind(spelling).expect("diagnostic source")).expect("offset");
            assert_eq!(
                first[0].primary_span().map(|span| (span.start(), span.end())),
                Some((start, start + u32::try_from(spelling.len()).expect("length")))
            );
        }
    }
}

fn fixture(
    vector: bool,
    exclusive: bool,
    replace: bool,
    rhs: &str,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut f, declarations, _) = initial(&Element::String);
    let ty = if vector { Ty::Vec(Box::new(Ty::String)) } else { Ty::String };
    let start = f.source.len();
    let function_span = f.text("function");
    f.text(" ");
    let name = f.name("observe");
    f.text("(");
    let parameters = vec![f.parameter("incoming", &ty)];
    f.text("): ");
    let result_type = f.ty(&ty);
    f.text(" ");
    let open_brace_span = f.text("{");
    f.text(" ");
    if rhs != "direct" {
        f.local("items", &ty, true, "incoming");
    }
    let nested = u32::try_from(f.statements.len()).expect("nested statement");
    let inner_open = f.text("{");
    f.statements
        .push(RawStatementSyntax { span: inner_open, kind: RawStatementKind::Block { block: 1 } });
    f.text(" ");
    alias(&mut f, &ty, exclusive, rhs);
    action(&mut f, &ty, replace, rhs);
    let inner_close = f.text("}");
    f.statements[nested as usize].span.end = inner_close.end;
    f.text(" ");
    let return_start = f.source.len();
    let keyword_span = f.text("return");
    f.text(" ");
    let value = f.reference(if rhs == "direct" { "incoming" } else { "items" });
    let semicolon_span = f.text(";");
    f.statements.push(RawStatementSyntax {
        span: at(return_start, f.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    f.text(" ");
    let close_brace_span = f.text("}");
    let body_span = at(open_brace_span.start as usize, f.source.len());
    let function = RawFunctionSyntax {
        span: at(start, f.source.len()),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span,
                    statements: (0..nested).chain([nested, nested + 3]).collect(),
                    close_brace_span,
                },
                RawBlockSyntax {
                    span: at(inner_open.start as usize, inner_close.end as usize),
                    open_brace_span: inner_open,
                    statements: vec![nested + 1, nested + 2],
                    close_brace_span: inner_close,
                },
            ],
            statements: f.statements,
            expressions: f.expressions,
        },
    };
    (
        f.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: f.types,
                data_declarations: declarations,
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[test]
fn nonindexed_owned_roots_clone_replace_and_restore() {
    for vector in [false, true] {
        for exclusive in [false, true] {
            for replace in [false, true] {
                let (source, raw) = fixture(vector, exclusive, replace, "loan");
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated source");
                if replace && !exclusive {
                    let first = lower(pair_input(&syntax, &sources)).expect_err("shared mutation");
                    let second =
                        lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection");
                    assert_eq!(first, second);
                    assert_eq!(first.len(), 1);
                    assert_eq!(first[0].code(), "ZRYNA-M3017");
                    assert_eq!(
                        first[0].message(),
                        "replacement through a borrow requires active exclusive authority"
                    );
                    assert_eq!(first[0].guidance(), "assign through a live BorrowMut alias");
                    let start = u32::try_from(source.find("clone(loan)").expect("RHS source"))
                        .expect("offset");
                    assert_eq!(
                        first[0].primary_span().map(|span| (span.start(), span.end())),
                        Some((start, start + 11))
                    );
                    continue;
                }
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let instructions =
                    function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                let begin = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
                    .expect("static begin");
                let cloned = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::GenericCloneBorrow)
                    .expect("retained alias clone");
                let end = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .expect("lexical restoration");
                assert!(begin < cloned && cloned < end);
                if replace {
                    let commit = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::BorrowReplace)
                        .expect("owned replacement");
                    assert!(cloned < commit && commit < end);
                }
                assert!(
                    instructions[end + 1..]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::MoveFromPlace)
                );
            }
        }
    }
}
