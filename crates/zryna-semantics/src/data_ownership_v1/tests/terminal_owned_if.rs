use super::*;

#[allow(clippy::too_many_lines)]
fn terminal_string_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};
    let text = "function choose(flag: bool): String { if (flag) { return \"a\"; } else { return \"b\"; } }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_span = token("String", 0);
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: 1,
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![1],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![2],
                    close_brace_span: token("}", 1),
                },
            ],
            statements: vec![
                RawStatementSyntax {
                    span: range("if", 0, "}", 1),
                    kind: RawStatementKind::If {
                        keyword_span: token("if", 0),
                        open_paren_span: token("(", 1),
                        condition: 0,
                        close_paren_span: token(")", 1),
                        then_block: 1,
                        else_clause: Some(RawElseSyntax {
                            keyword_span: token("else", 0),
                            block: 2,
                        }),
                    },
                },
                RawStatementSyntax {
                    span: range("return", 0, ";", 0),
                    kind: RawStatementKind::Return {
                        keyword_span: token("return", 0),
                        value: 1,
                        semicolon_span: token(";", 0),
                    },
                },
                RawStatementSyntax {
                    span: range("return", 1, ";", 1),
                    kind: RawStatementKind::Return {
                        keyword_span: token("return", 1),
                        value: 2,
                        semicolon_span: token(";", 1),
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: token("flag", 1),
                    kind: RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: token("flag", 1),
                        },
                    },
                },
                RawExpressionSyntax {
                    span: token("\"a\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
                },
                RawExpressionSyntax {
                    span: token("\"b\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
                },
            ],
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: vec![
                    RawTypeSyntax {
                        span: bool_span,
                        kind: RawTypeSyntaxKind::Named {
                            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
                        },
                    },
                    RawTypeSyntax {
                        span: string_span,
                        kind: RawTypeSyntaxKind::String { keyword_span: string_span },
                    },
                ],
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn terminal_vec_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};
    let text = "function choose(flag: bool): Vec<i32> { if (flag) { return Vec<i32>([1]); } else { return Vec<i32>([2]); } }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let mut types = vec![RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    }];
    let mut vec_types = Vec::new();
    for ordinal in 0..3 {
        let full = token("Vec<i32>", ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start, end: full.start + 3 };
        let less_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 3, end: full.start + 4 };
        let element_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 4, end: full.start + 7 };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: full.start + 7, end: full.end };
        let argument = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: full,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        vec_types.push(vec_id);
    }
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let expressions = vec![
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: token("1", 0),
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 1, ")", 2),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[1],
                open_paren_span: token("(", 2),
                open_bracket_span: token("[", 0),
                elements: vec![1],
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 2),
            },
        },
        RawExpressionSyntax {
            span: token("2", 3),
            kind: RawExpressionKind::I32Literal { spelling: "2".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 2, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[2],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: vec![3],
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("if", 0, "}", 1),
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 1),
                condition: 0,
                close_paren_span: token(")", 1),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 0),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 2,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("return", 1, ";", 1),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 1),
                value: 4,
                semicolon_span: token(";", 1),
            },
        },
    ];
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "choose".to_owned(), span: token("choose", 0) },
        parameters: vec![RawParameterSyntax {
            span: range("flag", 0, "bool", 0),
            name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 0) },
            type_syntax: 0,
        }],
        result_type: vec_types[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![1],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![2],
                    close_brace_span: token("}", 1),
                },
            ],
            statements,
            expressions,
        },
    };
    (
        text,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: Vec::new(),
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

#[test]
fn terminal_string_if_joins_owned_results_through_one_block_parameter() {
    let (source, raw) = terminal_string_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful terminal String if");
    let program = lower(pair_input(&syntax, &sources)).expect("terminal String phi must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let branch = blocks[0].terminator();
    assert_eq!(branch.value_operands().next().expect("condition").index(), 1);
    let (when_true, when_false) = branch.branch_edges().expect("entry branch");
    assert_eq!((when_true.target().index(), when_false.target().index()), (1, 2));
    assert_eq!((when_true.arguments().count(), when_false.arguments().count()), (0, 0));
    let then_value =
        blocks[1].instructions().next().expect("then String").result().expect("result");
    let else_value =
        blocks[2].instructions().next().expect("else String").result().expect("result");
    assert_eq!((then_value.index(), else_value.index()), (2, 3));
    let then_jump = blocks[1].terminator().edges().next().expect("then jump");
    let else_jump = blocks[2].terminator().edges().next().expect("else jump");
    assert_eq!((then_jump.target().index(), else_jump.target().index()), (3, 3));
    assert_eq!(then_jump.arguments().next(), Some(then_value));
    assert_eq!(else_jump.arguments().next(), Some(else_value));
    let joined = blocks[3].parameters().next().expect("String join parameter").id();
    assert_eq!(joined.index(), 4);
    assert_eq!(blocks[3].instructions().count(), 0);
    assert_eq!(blocks[3].terminator().value_operands().next(), Some(joined));
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
    assert!(function.places().any(
        |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == joined)
    ));
    assert_eq!(
        function
            .places()
            .filter(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(_)))
            .count(),
        3
    );
    assert_eq!(function.cleanup_plans().last().expect("join return cleanup").actions().count(), 0);
}

#[test]
fn terminal_vec_if_joins_exact_vec_results_through_one_block_parameter() {
    let (source, raw) = terminal_vec_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful terminal Vec if");
    let program = lower(pair_input(&syntax, &sources)).expect("terminal Vec phi must verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(function.parameters().next().expect("bool parameter").id().index(), 0);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let branch = blocks[0].terminator();
    assert_eq!(branch.value_operands().next().expect("condition").index(), 1);
    let (when_true, when_false) = branch.branch_edges().expect("entry branch");
    assert_eq!((when_true.target().index(), when_false.target().index()), (1, 2));
    assert_eq!((when_true.arguments().count(), when_false.arguments().count()), (0, 0));
    let then_results = blocks[1]
        .instructions()
        .map(|instruction| instruction.result().expect("then result").index())
        .collect::<Vec<_>>();
    let else_results = blocks[2]
        .instructions()
        .map(|instruction| instruction.result().expect("else result").index())
        .collect::<Vec<_>>();
    assert_eq!(then_results, vec![2, 3]);
    assert_eq!(else_results, vec![4, 5]);
    let then_jump = blocks[1].terminator().edges().next().expect("then jump");
    let else_jump = blocks[2].terminator().edges().next().expect("else jump");
    assert_eq!((then_jump.target().index(), else_jump.target().index()), (3, 3));
    assert_eq!(
        then_jump.arguments().next().map(zryna_ir::data_ownership_v1::ValueIdentity::index),
        Some(3)
    );
    assert_eq!(
        else_jump.arguments().next().map(zryna_ir::data_ownership_v1::ValueIdentity::index),
        Some(5)
    );
    let joined = blocks[3].parameters().next().expect("owned Vec join parameter").id();
    assert_eq!(joined.index(), 6);
    assert!(function.places().any(|place| {
        matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == joined)
    }));
    assert_eq!(blocks[3].terminator().value_operands().next(), Some(joined));
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
    assert_eq!(function.cleanup_plans().last().expect("join return cleanup").actions().count(), 0);
}

#[test]
fn terminal_owned_if_rejects_missing_else_and_arm_fallthrough() {
    let (source, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    let RawStatementKind::If { else_clause, .. } = &mut function.body.statements[0].kind else {
        unreachable!("fixture root if")
    };
    *else_clause = None;
    let sources = sources_for(&source);
    let mut errors = Errors::new(&sources);
    assert!(terminal_owned_if(function, &sources, &mut errors).is_none());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    let missing_else_span = diagnostics[0].primary_span().expect("missing else span");
    let expected = untrusted_range(&source, ("if", 0), ("}", 1));
    assert_eq!(
        (missing_else_span.start(), missing_else_span.end()),
        (expected.start, expected.end)
    );

    let (source, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    let RawStatementKind::Return { value, semicolon_span, .. } = function.body.statements[1].kind
    else {
        unreachable!("fixture then return")
    };
    function.body.statements[1].kind =
        RawStatementKind::ExpressionStatement { expression: value, semicolon_span };
    let sources = sources_for(&source);
    let mut errors = Errors::new(&sources);
    assert!(terminal_owned_if(function, &sources, &mut errors).is_none());
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    let fallthrough_span = diagnostics[0].primary_span().expect("fallthrough span");
    let expected = untrusted_range(&source, ("return", 0), (";", 0));
    assert_eq!((fallthrough_span.start(), fallthrough_span.end()), (expected.start, expected.end));
}

#[test]
fn terminal_owned_phi_routing_is_narrowly_private_string_or_vec() {
    let (_, mut raw) = terminal_string_if_fixture();
    let function = &mut raw.files[0].functions[0];
    assert!(is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, false));
    assert!(is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::Vec, true));
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, true));
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::Bool, false));
    assert!(!is_terminal_owned_phi_candidate(
        function,
        zryna_layout::TypeCategory::FixedArray,
        false,
    ));
    function.export_span = Some(function.function_span);
    assert!(!is_terminal_owned_phi_candidate(function, zryna_layout::TypeCategory::String, false));
}
