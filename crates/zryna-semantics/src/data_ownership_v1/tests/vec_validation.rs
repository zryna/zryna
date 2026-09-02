use super::*;

fn unresolved_push_without_vec_snapshot() -> RawProjectSyntaxSnapshot {
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![RawTypeSyntax {
                span: s(16, 19),
                kind: RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: "i32".to_owned(), span: s(16, 19) },
                },
            }],
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: s(0, 51),
                export_span: None,
                function_span: s(0, 8),
                name: RawIdentifierSyntax { text: "bad".to_owned(), span: s(9, 12) },
                parameters: Vec::new(),
                result_type: 0,
                body: RawFunctionBodySyntax {
                    span: s(20, 51),
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: s(20, 51),
                        open_brace_span: s(20, 21),
                        statements: vec![0, 1],
                        close_brace_span: s(50, 51),
                    }],
                    statements: vec![
                        RawStatementSyntax {
                            span: s(22, 39),
                            kind: RawStatementKind::ExpressionStatement {
                                expression: 2,
                                semicolon_span: s(38, 39),
                            },
                        },
                        RawStatementSyntax {
                            span: s(40, 49),
                            kind: RawStatementKind::Return {
                                keyword_span: s(40, 46),
                                value: 3,
                                semicolon_span: s(48, 49),
                            },
                        },
                    ],
                    expressions: vec![
                        RawExpressionSyntax {
                            span: s(27, 34),
                            kind: zryna_syntax::v4::RawExpressionKind::Reference {
                                name: RawIdentifierSyntax {
                                    text: "missing".to_owned(),
                                    span: s(27, 34),
                                },
                            },
                        },
                        RawExpressionSyntax {
                            span: s(36, 37),
                            kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                                spelling: "1".to_owned(),
                            },
                        },
                        RawExpressionSyntax {
                            span: s(22, 38),
                            kind: zryna_syntax::v4::RawExpressionKind::VecPush {
                                keyword_span: s(22, 26),
                                open_paren_span: s(26, 27),
                                vector: 0,
                                comma_span: s(34, 35),
                                value: 1,
                                close_paren_span: s(37, 38),
                            },
                        },
                        RawExpressionSyntax {
                            span: s(47, 48),
                            kind: zryna_syntax::v4::RawExpressionKind::I32Literal {
                                spelling: "0".to_owned(),
                            },
                        },
                    ],
                },
            }],
        }],
        diagnostics: Vec::new(),
    }
}
#[test]
fn private_vec_assignment_rejects_a_copy_typed_source() {
    let source = VEC_ASSIGN_I32_SOURCE.replacen(
        "const y: Vec<i32> = Vec<i32>([]);",
        "const y: i32      = 0           ;",
        1,
    );
    let sources = sources_for(&source);
    let mut raw = response_snapshot(VEC_ASSIGN_I32_RESPONSE);
    raw.files[0].type_syntax.truncate(6);
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 66, end: 69 },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "i32".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 66, end: 69 },
            },
        },
    });
    let RawStatementKind::LocalDeclaration { type_syntax, .. } =
        &mut raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("second local")
    };
    *type_syntax = 6;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 77, end: 78 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Copy mismatch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Copy assignment source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}

#[test]
fn private_vec_assignment_rejects_a_projection_target() {
    let source = VEC_ASSIGN_STRING_SOURCE.replacen("x = Vec", "x[0] = Vec", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot(response_snapshot(VEC_ASSIGN_STRING_RESPONSE), 71, 3);
    let body = &mut raw.files[0].functions[0].body;
    let initial_literal = body.expressions[0].clone();
    let initial_vec = body.expressions[1].clone();
    let rhs_literal = body.expressions[3].clone();
    let mut rhs_vec = body.expressions[4].clone();
    let returned = body.expressions[5].clone();
    let base = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    let index = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 71, end: 72 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    };
    let target = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 73 },
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 2,
            open_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 70, end: 71 },
            index: 3,
            close_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 72, end: 73 },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::VecConstruction { elements, .. } = &mut rhs_vec.kind
    else {
        panic!("RHS Vec")
    };
    elements[0] = 5;
    body.expressions =
        vec![initial_literal, initial_vec, base, index, target, rhs_literal, rhs_vec, returned];
    let RawStatementKind::Assignment { target, value, .. } = &mut body.statements[1].kind else {
        panic!("assignment")
    };
    *target = 4;
    *value = 6;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 7;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projection target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("projection assignment");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3013")
        .expect("projection target diagnostic");
    let at = diagnostic.primary_span().expect("projection span");
    assert_eq!((at.start(), at.end()), (69, 73));
}

#[test]
fn private_vec_push_rejects_immutable_target() {
    let source = VEC_PUSH_SOURCE.replacen("let values", "const values", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 36, 2);
    raw.files[0].functions[0].body.statements[0].kind =
        match &raw.files[0].functions[0].body.statements[0].kind {
            RawStatementKind::LocalDeclaration {
                keyword_span,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
                ..
            } => RawStatementKind::LocalDeclaration {
                keyword_span: *keyword_span,
                mutable: false,
                name: name.clone(),
                type_syntax: *type_syntax,
                equals_span: *equals_span,
                initializer: *initializer,
                semicolon_span: *semicolon_span,
            },
            _ => panic!("local"),
        };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable push");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable push");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3014"));
}

#[test]
fn private_vec_push_rejects_wrong_element_type() {
    let source = VEC_PUSH_SOURCE.replacen("\"b\"", "1", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 95, -2);
    raw.files[0].functions[0].body.expressions[3].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong push element");
    let first = lower(pair_input(&syntax, &sources)).expect_err("wrong push element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic wrong push element");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, nth_untrusted_span(&source, "1", 0))));
}

#[test]
fn private_vec_construct_rejects_unsupported_string_element_at_exact_span_deterministically() {
    let source = VEC_PUSH_SOURCE.replacen("\"a\"", "1", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_PUSH_RESPONSE), 75, -2);
    raw.files[0].functions[0].body.expressions[0].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong construct element");
    let first = lower(pair_input(&syntax, &sources)).expect_err("wrong construct element");
    let second =
        lower(pair_input(&syntax, &sources)).expect_err("deterministic wrong construct element");
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3013");
    assert_eq!(first[0].primary_span(), Some(span(&sources, nth_untrusted_span(&source, "1", 0))));
}

#[test]
fn private_vec_in_range_positive_index_uses_same_checked_cleanup() {
    let source = VEC_INDEX_SOURCE.replacen("[-1]", "[1]", 1);
    let sources = sources_for(&source);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let mut raw = shift_snapshot_signed(response_snapshot(&response), 83, -1);
    raw.files[0].functions[0].body.expressions[4].kind =
        zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "1".to_owned() };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful positive Vec index");
    let program = lower(pair_input(&syntax, &sources)).expect("in-range Vec index");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(block.terminator().derived_drop_actions().count(), 1);
}

#[test]
fn unresolved_or_wrong_case_vec_push_and_index_names_are_source_errors() {
    for replacement in ["absent", "Values"] {
        let source = VEC_PUSH_SOURCE.replacen("push(values", &format!("push({replacement}"), 1);
        let sources = sources_for(&source);
        let mut raw = response_snapshot(VEC_PUSH_RESPONSE);
        let function = &mut raw.files[0].functions[0];
        let vector = function
            .body
            .expressions
            .iter()
            .find_map(|expression| match expression.kind {
                zryna_syntax::v4::RawExpressionKind::VecPush { vector, .. } => Some(vector),
                _ => None,
            })
            .expect("push target");
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut function.body.expressions[usize::try_from(vector).expect("vector index")].kind
        else {
            panic!("push reference")
        };
        name.text = replacement.to_owned();
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful bad push target");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("bad push target");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
    }

    let source = VEC_INDEX_SOURCE.replacen("return values[", "return Values[", 1);
    let sources = sources_for(&source);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let mut raw = response_snapshot(&response);
    let function = &mut raw.files[0].functions[0];
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut function.body.expressions[3].kind
    else {
        panic!("index base")
    };
    name.text = "Values".to_owned();
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong-case index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong-case index");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
}

#[test]
fn routed_push_without_any_vec_type_cannot_disappear_silently() {
    const SOURCE: &str = "function bad(): i32 { push(missing, 1); return 0; }";
    let sources = sources_for(SOURCE);
    let syntax = verify_snapshot(unresolved_push_without_vec_snapshot(), &sources)
        .expect("source-faithful unresolved push v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing Vec type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}

#[test]
fn private_vec_local_names_reject_exact_and_ascii_fold_collisions() {
    for replacement in ["values", "VALUES"] {
        let source = VEC_STRING_SOURCE.replace("first", replacement);
        let sources = sources_for(&source);
        let mut raw = shift_snapshot(response_snapshot(VEC_STRING_RESPONSE), 42, 1);
        raw = shift_snapshot(raw, 105, 1);
        let function = &mut raw.files[0].functions[0];
        let RawStatementKind::LocalDeclaration { name, .. } = &mut function.body.statements[0].kind
        else {
            panic!("first local")
        };
        name.text = replacement.to_owned();
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut function.body.expressions[1].kind
        else {
            panic!("first reference")
        };
        name.text = replacement.to_owned();
        let syntax = match verify_snapshot(raw, &sources) {
            Ok(syntax) => syntax,
            Err(diagnostics) if replacement == "values" => {
                assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-Y4002"));
                continue;
            }
            Err(diagnostics) => panic!("source-faithful colliding Vec local: {diagnostics:?}"),
        };
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("colliding Vec local");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3002"));
    }
}

#[test]
fn vec_push_target_guard_rejects_moved_and_immutable_states() {
    assert!(!vec_push_target_invalid(true, true));
    assert!(vec_push_target_invalid(false, true));
    assert!(vec_push_target_invalid(true, false));
}

#[test]
fn private_vec_string_indexing_is_rejected_by_copy_only_rule() {
    let source = VEC_STRING_SOURCE.replacen("return values;", "return values[0];", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(VEC_STRING_RESPONSE), 126, 3);
    let function = &mut raw.files[0].functions[0];
    function.body.expressions[4].span.end = 126;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut function.body.expressions[4].kind
    else {
        panic!("values reference")
    };
    name.span.end = 126;
    function.body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 127, end: 128 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    function.body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 120, end: 129 },
        kind: zryna_syntax::v4::RawExpressionKind::Index {
            base: 4,
            open_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 126, end: 127 },
            index: 5,
            close_bracket_span: zryna_source::UntrustedSpan { file: 0, start: 128, end: 129 },
        },
    });
    let RawStatementKind::Return { value, .. } = &mut function.body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> index");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("String index excluded");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3013"));
}
