use super::*;

#[path = "nonindexed_static_owned_borrow/calls.rs"]
mod calls;

#[derive(Clone, Copy)]
pub(super) enum BorrowedPlace {
    StructRoot,
    StructField,
    ArrayRoot,
    ArrayElement,
}

fn root_type(place: BorrowedPlace) -> Ty {
    match place {
        BorrowedPlace::StructRoot | BorrowedPlace::StructField => Ty::Named("Parcel"),
        BorrowedPlace::ArrayRoot | BorrowedPlace::ArrayElement => {
            Ty::Array(Box::new(Ty::String), 2)
        }
    }
}

fn referent_type(place: BorrowedPlace) -> Ty {
    match place {
        BorrowedPlace::StructRoot => Ty::Named("Parcel"),
        BorrowedPlace::StructField => Ty::Vec(Box::new(Ty::String)),
        BorrowedPlace::ArrayRoot => Ty::Array(Box::new(Ty::String), 2),
        BorrowedPlace::ArrayElement => Ty::String,
    }
}

fn borrowed_place(builder: &mut Builder, place: BorrowedPlace) -> u32 {
    let start = builder.source.len();
    let base = builder.reference("items");
    match place {
        BorrowedPlace::StructRoot | BorrowedPlace::ArrayRoot => base,
        BorrowedPlace::StructField => {
            let dot_span = builder.text(".");
            let field = builder.name("value");
            builder.expression(start, RawExpressionKind::FieldAccess { base, dot_span, field })
        }
        BorrowedPlace::ArrayElement => {
            let open_bracket_span = builder.text("[");
            let index_start = builder.source.len();
            builder.text("0");
            let index = builder
                .expression(index_start, RawExpressionKind::I32Literal { spelling: "0".into() });
            let close_bracket_span = builder.text("]");
            builder.expression(
                start,
                RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
            )
        }
    }
}

fn alias(
    builder: &mut Builder,
    name_text: &str,
    place: BorrowedPlace,
    exclusive: bool,
    declared: Option<Ty>,
) {
    let start = builder.source.len();
    let keyword_span = builder.text("const");
    builder.text(" ");
    let name = builder.name(name_text);
    builder.text(": ");
    let type_start = builder.source.len();
    let borrow_keyword = builder.text(if exclusive { "BorrowMut" } else { "Borrow" });
    let less_than_span = builder.text("<");
    let argument = builder.ty(&declared.unwrap_or_else(|| referent_type(place)));
    let greater_than_span = builder.text(">");
    let type_syntax = u32::try_from(builder.types.len()).expect("borrow type");
    builder.types.push(RawTypeSyntax {
        span: at(type_start, builder.source.len()),
        kind: if exclusive {
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
        },
    });
    builder.text(" = ");
    let borrow_start = builder.source.len();
    let borrow_keyword = builder.text(if exclusive { "borrowMut" } else { "borrow" });
    let open_paren_span = builder.text("(");
    let value = borrowed_place(builder, place);
    let close_paren_span = builder.text(")");
    let initializer = builder.expression(
        borrow_start,
        if exclusive {
            RawExpressionKind::BorrowMut {
                keyword_span: borrow_keyword,
                open_paren_span,
                value,
                close_paren_span,
            }
        } else {
            RawExpressionKind::Borrow {
                keyword_span: borrow_keyword,
                open_paren_span,
                value,
                close_paren_span,
            }
        },
    );
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(start, builder.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span: at(borrow_start - 2, borrow_start - 1),
            initializer,
            semicolon_span,
        },
    });
    builder.text(" ");
}

fn operation(builder: &mut Builder, place: BorrowedPlace, replace: bool) {
    let start = builder.source.len();
    let kind = if replace {
        let target = builder.reference("loan");
        builder.text(" ");
        let equals_span = builder.text("=");
        builder.text(" ");
        let value = builder.clone_value(|builder| builder.reference("loan"));
        RawStatementKind::Assignment {
            target,
            equals_span,
            value,
            semicolon_span: builder.text(";"),
        }
    } else {
        let keyword_span = builder.text("const");
        builder.text(" seen: ");
        let name = RawIdentifierSyntax { text: "seen".into(), span: at(start + 6, start + 10) };
        let type_syntax = builder.ty(&referent_type(place));
        builder.text(" ");
        let equals_span = builder.text("=");
        builder.text(" ");
        let initializer = builder.clone_value(|builder| builder.reference("loan"));
        RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span: builder.text(";"),
        }
    };
    builder.statements.push(RawStatementSyntax { span: at(start, builder.source.len()), kind });
    builder.text(" ");
}

fn move_projection(builder: &mut Builder, place: BorrowedPlace) {
    let start = builder.source.len();
    let keyword_span = builder.text("const");
    builder.text(" spent: ");
    let name = RawIdentifierSyntax { text: "spent".into(), span: at(start + 6, start + 11) };
    let type_syntax = builder.ty(&referent_type(place));
    builder.text(" ");
    let equals_span = builder.text("=");
    builder.text(" ");
    let initializer = borrowed_place(builder, place);
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(start, builder.source.len()),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span,
            mutable: false,
            name,
            type_syntax,
            equals_span,
            initializer,
            semicolon_span,
        },
    });
    builder.text(" ");
}

fn fixture(
    place: BorrowedPlace,
    exclusive: bool,
    replace: bool,
    moved: bool,
    declared: Option<Ty>,
    prior: Option<BorrowedPlace>,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut builder, declarations, _) =
        initial(if matches!(place, BorrowedPlace::StructRoot | BorrowedPlace::StructField) {
            &Element::Struct
        } else {
            &Element::String
        });
    let root = root_type(place);
    builder.text("\n");
    let function_start = builder.source.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("observe");
    builder.text("(");
    let parameters = vec![builder.parameter("incoming", &root)];
    builder.text("): ");
    let result_type = builder.ty(&root);
    builder.text(" {");
    let open_brace_span = at(builder.source.len() - 1, builder.source.len());
    builder.text(" ");
    builder.local("items", &root, true, "incoming");
    if moved {
        move_projection(&mut builder, place);
    }
    let nested = u32::try_from(builder.statements.len()).expect("nested statement");
    let inner_open = builder.text("{");
    builder
        .statements
        .push(RawStatementSyntax { span: inner_open, kind: RawStatementKind::Block { block: 1 } });
    builder.text(" ");
    let nested_first = nested + 1;
    if let Some(prior) = prior {
        alias(&mut builder, "guard", prior, false, None);
    }
    alias(&mut builder, "loan", place, exclusive, declared);
    operation(&mut builder, place, replace);
    let nested_end = u32::try_from(builder.statements.len()).expect("nested statements");
    let inner_close = builder.text("}");
    builder.statements[nested as usize].span.end = inner_close.end;
    builder.text(" return ");
    let keyword_span = at(builder.source.len() - 7, builder.source.len() - 1);
    let value = builder.reference("items");
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: at(keyword_span.start as usize, builder.source.len()),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
    builder.text(" }");
    let close_brace_span = at(builder.source.len() - 1, builder.source.len());
    let body_span = at(open_brace_span.start as usize, close_brace_span.end as usize);
    let return_id = u32::try_from(builder.statements.len() - 1).expect("return statement");
    let function = RawFunctionSyntax {
        span: at(function_start, builder.source.len()),
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
                    statements: (0..nested).chain([nested, return_id]).collect(),
                    close_brace_span,
                },
                RawBlockSyntax {
                    span: at(inner_open.start as usize, inner_close.end as usize),
                    open_brace_span: inner_open,
                    statements: (nested_first..nested_end).collect(),
                    close_brace_span: inner_close,
                },
            ],
            statements: builder.statements,
            expressions: builder.expressions,
        },
    };
    (
        builder.source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".into(),
                imports: Vec::new(),
                type_syntax: builder.types,
                data_declarations: declarations,
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

pub(super) fn resource_fixture(place: BorrowedPlace) -> (String, RawProjectSyntaxSnapshot) {
    fixture(place, false, false, false, None, None)
}

#[test]
fn nonindexed_owned_struct_and_array_places_clone_replace_and_restore() {
    for place in [
        BorrowedPlace::StructRoot,
        BorrowedPlace::StructField,
        BorrowedPlace::ArrayRoot,
        BorrowedPlace::ArrayElement,
    ] {
        for exclusive in [false, true] {
            let (source, raw) = fixture(place, exclusive, exclusive, false, None, None);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated static borrow");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions =
                function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
            let begin = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
                .expect("lexical begin");
            let clone = instructions
                .iter()
                .position(|instruction| {
                    instruction.kind() == VerifiedInstructionKind::GenericCloneBorrow
                })
                .expect("owned clone through borrow");
            let end = instructions
                .iter()
                .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
                .expect("lexical end");
            assert!(begin < clone && clone < end);
            let borrowed = instructions[begin].place_operands().next().expect("borrowed place");
            let borrowed_place =
                function.places().find(|place| place.id() == borrowed).expect("place view");
            assert_eq!(
                borrowed_place.ty(),
                instructions[clone].generic_clone().expect("clone").ty()
            );
            assert!(instructions[clone].derived_drop_actions().any(|action| {
                action.root()
                    == function
                        .places()
                        .find(|place| place.kind() == VerifiedPlaceKind::Local(0))
                        .expect("retained root")
                        .id()
            }));
            if exclusive {
                let replacement = instructions
                    .iter()
                    .find_map(|instruction| instruction.borrow_replacement())
                    .expect("exclusive replacement");
                assert_eq!(replacement.referent(), borrowed_place.ty());
            }
            assert!(instructions[end + 1..]
                .iter()
                .any(|instruction| instruction.kind() == VerifiedInstructionKind::MoveFromPlace));
            let replay = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
        }
    }
}

#[test]
fn nonindexed_owned_static_places_reject_partial_wrong_mode_and_wrong_type_then_recover() {
    for (place, exclusive, replace, moved, declared, expected, spelling) in [
        (BorrowedPlace::StructField, false, true, false, None, "ZRYNA-M3017", "clone(loan)"),
        (BorrowedPlace::ArrayElement, true, false, true, None, "ZRYNA-M3014", "items[0]"),
        (
            BorrowedPlace::StructField,
            false,
            false,
            false,
            Some(Ty::String),
            "ZRYNA-M3017",
            "items.value",
        ),
    ] {
        let (source, raw) = fixture(place, exclusive, replace, moved, declared, None);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid static borrow");
        let first = lower(pair_input(&syntax, &sources)).expect_err("invalid static borrow");
        assert_eq!(first.len(), 1, "{source}");
        assert_eq!(first[0].code(), expected, "{source}");
        let start = u32::try_from(source.rfind(spelling).expect("diagnostic source"))
            .expect("source offset");
        assert_eq!(
            first[0].primary_span().map(|span| (span.start(), span.end())),
            Some((start, start + u32::try_from(spelling.len()).expect("source length")))
        );
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources)).expect_err("deterministic static rejection")
        );

        let (valid_source, valid_raw) = fixture(place, false, false, false, None, None);
        let valid_sources = sources_for(&valid_source);
        let valid_syntax =
            verify_snapshot(valid_raw, &valid_sources).expect("authenticated recovery source");
        lower(pair_input(&valid_syntax, &valid_sources)).expect("valid static borrow recovery");
    }
}

#[test]
fn nonindexed_owned_parent_and_subobject_borrows_overlap_exactly() {
    for (root, projection) in [
        (BorrowedPlace::StructRoot, BorrowedPlace::StructField),
        (BorrowedPlace::ArrayRoot, BorrowedPlace::ArrayElement),
    ] {
        let (source, raw) = fixture(projection, true, false, false, None, Some(root));
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated overlapping borrows");
        let first = lower(pair_input(&syntax, &sources)).expect_err("overlap rejected");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), "ZRYNA-M3014");
        assert_eq!(first[0].message(), "owner access conflicts with an active borrow");
        let spelling = match projection {
            BorrowedPlace::StructField => "items.value",
            BorrowedPlace::ArrayElement => "items[0]",
            _ => unreachable!("projected overlap case"),
        };
        let start =
            u32::try_from(source.rfind(spelling).expect("overlap source")).expect("source offset");
        assert_eq!(
            first[0].primary_span().map(|span| (span.start(), span.end())),
            Some((start, start + u32::try_from(spelling.len()).expect("source length")))
        );
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources)).expect_err("deterministic overlap rejection")
        );
    }
}
