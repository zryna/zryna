use super::*;

#[allow(clippy::too_many_lines)]
fn private_string_if_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let text = "function choose(flag: bool): String { const own: String = \"keep\"; if (flag) { const first: String = \"a\"; const second: String = \"b\"; } else { const third: String = clone(own); } return own; }".to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..5).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
    let types = std::iter::once(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    })
    .chain(string_spans.iter().copied().map(|keyword_span| RawTypeSyntax {
        span: keyword_span,
        kind: RawTypeSyntaxKind::String { keyword_span },
    }))
    .collect();
    let root_open = token("{", 0);
    let then_open = token("{", 1);
    let else_open = token("{", 2);
    let then_close = token("}", 0);
    let else_close = token("}", 1);
    let root_close = token("}", 2);
    let outer_statement = range("const own", 0, ";", 0);
    let if_statement = range("if", 0, "}", 1);
    let first_statement = range("const first", 0, ";", 1);
    let second_statement = range("const second", 0, ";", 2);
    let third_statement = range("const third", 0, ";", 3);
    let return_statement = range("return", 0, ";", 4);
    let expressions = vec![
        RawExpressionSyntax {
            span: token("\"keep\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"keep\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
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
        RawExpressionSyntax {
            span: token("own", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 1) },
            },
        },
        RawExpressionSyntax {
            span: range("clone", 0, ")", 2),
            kind: RawExpressionKind::Clone {
                keyword_span: token("clone", 0),
                open_paren_span: token("(", 2),
                value: 4,
                close_paren_span: token(")", 2),
            },
        },
        RawExpressionSyntax {
            span: token("own", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 2) },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: outer_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: if_statement,
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 1),
                condition: 1,
                close_paren_span: token(")", 1),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: first_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: 3,
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: second_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: 4,
                equals_span: token("=", 2),
                initializer: 3,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: third_statement,
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 3),
                mutable: false,
                name: RawIdentifierSyntax { text: "third".to_owned(), span: token("third", 0) },
                type_syntax: 5,
                equals_span: token("=", 3),
                initializer: 5,
                semicolon_span: token(";", 3),
            },
        },
        RawStatementSyntax {
            span: return_statement,
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 6,
                semicolon_span: token(";", 4),
            },
        },
    ];
    let body_span =
        zryna_source::UntrustedSpan { file: 0, start: root_open.start, end: root_close.end };
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
            span: body_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: root_open,
                    statements: vec![0, 1, 5],
                    close_brace_span: root_close,
                },
                RawBlockSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: then_open.start,
                        end: then_close.end,
                    },
                    open_brace_span: then_open,
                    statements: vec![2, 3],
                    close_brace_span: then_close,
                },
                RawBlockSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: else_open.start,
                        end: else_close.end,
                    },
                    open_brace_span: else_open,
                    statements: vec![4],
                    close_brace_span: else_close,
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

fn private_string_if_moves_outer_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;

    let (source, mut raw) = private_string_if_fixture();
    let source = source.replacen("\"a\"", "own", 1);
    let expression = &mut raw.files[0].functions[0].body.expressions[2];
    expression.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "own".to_owned(), span: expression.span },
    };
    (source, raw)
}

fn private_string_if_non_bool_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (source, mut raw) = private_string_if_fixture();
    let source = source.replacen("if (flag)", "if (own )", 1);
    let expression = &mut raw.files[0].functions[0].body.expressions[1];
    expression.span.end -= 1;
    expression.kind = zryna_syntax::v4::RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "own".to_owned(), span: expression.span },
    };
    (source, raw)
}

fn private_string_if_without_else_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = private_string_if_fixture();
    let clause = " else { const third: String = clone(own); }";
    let start = source.find(clause).expect("else clause");
    source.replace_range(start..start + clause.len(), &" ".repeat(clause.len()));
    let body = &mut raw.files[0].functions[0].body;
    let then_end = body.blocks[1].close_brace_span.end;
    let RawStatementKind::If { else_clause, .. } = &mut body.statements[1].kind else {
        panic!("if statement")
    };
    *else_clause = None;
    body.statements[1].span.end = then_end;
    body.blocks.truncate(2);
    body.statements.remove(4);
    body.statements[4].kind = match body.statements[4].kind.clone() {
        RawStatementKind::Return { keyword_span, semicolon_span, .. } => {
            RawStatementKind::Return { keyword_span, value: 4, semicolon_span }
        }
        _ => panic!("return statement"),
    };
    body.blocks[0].statements = vec![0, 1, 4];
    body.expressions.drain(4..6);
    raw.files[0].type_syntax.pop();
    (source, raw)
}

fn private_string_if_nested_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let (mut source, mut raw) = private_string_if_fixture();
    let original = "const first: String = \"a\";";
    let nested = "if (true) { }";
    let start = source.find(original).expect("first branch local");
    let replacement = format!("{nested}{}", " ".repeat(original.len() - nested.len()));
    source.replace_range(start..start + original.len(), &replacement);
    let nested_keyword = nth_untrusted_span(&source, "if", 1);
    let nested_open_paren = nth_untrusted_span(&source, "(", 2);
    let nested_close_paren = nth_untrusted_span(&source, ")", 2);
    let nested_open = nth_untrusted_span(&source, "{", 2);
    let nested_close = nth_untrusted_span(&source, "}", 0);
    let nested_span =
        zryna_source::UntrustedSpan { file: 0, start: nested_keyword.start, end: nested_close.end };
    raw.files[0].type_syntax.remove(3);
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::If { else_clause: Some(RawElseSyntax { block, .. }), .. } =
        &mut body.statements[1].kind
    else {
        panic!("outer if")
    };
    *block = 3;
    body.statements[2] = RawStatementSyntax {
        span: nested_span,
        kind: RawStatementKind::If {
            keyword_span: nested_keyword,
            open_paren_span: nested_open_paren,
            condition: 2,
            close_paren_span: nested_close_paren,
            then_block: 2,
            else_clause: None,
        },
    };
    body.expressions[2] = RawExpressionSyntax {
        span: nth_untrusted_span(&source, "true", 0),
        kind: RawExpressionKind::BoolLiteral { value: true },
    };
    body.blocks.insert(
        2,
        RawBlockSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: nested_open.start,
                end: nested_close.end,
            },
            open_brace_span: nested_open,
            statements: Vec::new(),
            close_brace_span: nested_close,
        },
    );
    for statement in &mut body.statements[3..=4] {
        let RawStatementKind::LocalDeclaration { type_syntax, .. } = &mut statement.kind else {
            panic!("remaining branch local")
        };
        *type_syntax -= 1;
    }
    (source, raw)
}

#[test]
fn private_string_if_restores_outer_owner_and_drops_branch_locals_in_reverse() {
    let (source, raw) = private_string_if_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned String if");
    let program = lower(pair_input(&syntax, &sources)).expect("owned String if must verify");
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
    assert_eq!(blocks.iter().map(|block| block.id().index()).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let then_kinds = blocks[1]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        then_kinds,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let then_drops = blocks[1]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(then_drops, vec![6, 4]);
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringClone,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks
            .iter()
            .flat_map(|block| block.instructions())
            .filter_map(|instruction| {
                instruction.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            })
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    let plans = function
        .cleanup_plans()
        .map(|plan| {
            (
                plan.id().index(),
                plan.site().role(),
                plan.actions()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plans,
        vec![
            (0, VerifiedCleanupRole::PrepareFailure, vec![]),
            (1, VerifiedCleanupRole::PrepareFailure, vec![2]),
            (2, VerifiedCleanupRole::PrepareFailure, vec![4, 2]),
            (3, VerifiedCleanupRole::PrepareFailure, vec![2]),
            (4, VerifiedCleanupRole::Return, vec![]),
        ]
    );
    let cleanup_spans = function
        .cleanup_plans()
        .map(zryna_ir::data_ownership_v1::VerifiedCleanupPlan::span)
        .collect::<Vec<_>>();
    assert_eq!(cleanup_spans[1], span(&sources, nth_untrusted_span(&source, "\"a\"", 0)));
    assert_eq!(cleanup_spans[2], span(&sources, nth_untrusted_span(&source, "\"b\"", 0)));
    assert_eq!(cleanup_spans[3], span(&sources, untrusted_range(&source, ("clone", 0), (")", 2))));
}

#[test]
fn private_string_if_rejects_one_arm_moving_an_outer_owner() {
    let (source, raw) = private_string_if_moves_outer_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful outer move in branch");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("outer move must not join");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3015"));
}

#[test]
fn private_string_if_rejects_non_bool_reference_condition() {
    let (source, raw) = private_string_if_non_bool_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful i32 branch condition");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("i32 condition must reject");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3012"),
        "{diagnostics:?}"
    );
}

#[test]
fn private_string_if_without_else_synthesizes_empty_false_path() {
    let (source, raw) = private_string_if_without_else_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful if without else");
    let program = lower(pair_input(&syntax, &sources)).expect("omitted else must be empty path");
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
    assert_eq!(blocks[2].instructions().count(), 0);
    assert_eq!(blocks[2].terminator().kind(), VerifiedTerminatorKind::Jump);
}

#[test]
fn private_string_if_rejects_nested_owned_control_flow() {
    let (source, raw) = private_string_if_nested_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested owned if");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nested owned if rejects");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));
}
