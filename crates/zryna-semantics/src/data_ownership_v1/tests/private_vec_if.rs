use super::*;

#[allow(clippy::too_many_lines)]
fn private_vec_if_fixture(push_outer: bool, element: &str) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::{RawElseSyntax, RawExpressionKind};

    let target = if push_outer { "own" } else { "branch" };
    let pushed = if element == "String" { "\"p\"" } else { "7" };
    let text = format!(
        "function choose(flag: bool): Vec<{element}> {{ const own: Vec<{element}> = Vec<{element}>([]); if (flag) {{ let branch: Vec<{element}> = Vec<{element}>([]); push({target}, {pushed}); }} else {{ const value: String = \"e\"; }} return own; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let mut types = Vec::new();
    let bool_span = token("bool", 0);
    types.push(RawTypeSyntax {
        span: bool_span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "bool".to_owned(), span: bool_span },
        },
    });
    let mut vec_type_ids = Vec::new();
    let vec_spelling = format!("Vec<{element}>");
    for ordinal in 0..5 {
        let vec_span = token(&vec_spelling, ordinal);
        let keyword_span =
            zryna_source::UntrustedSpan { file: 0, start: vec_span.start, end: vec_span.start + 3 };
        let less_than_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 3,
            end: vec_span.start + 4,
        };
        let element_span = zryna_source::UntrustedSpan {
            file: 0,
            start: vec_span.start + 4,
            end: vec_span.start + 4 + u32::try_from(element.len()).expect("element length"),
        };
        let greater_than_span =
            zryna_source::UntrustedSpan { file: 0, start: element_span.end, end: vec_span.end };
        let element_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: element_span,
            kind: if element == "String" {
                RawTypeSyntaxKind::String { keyword_span: element_span }
            } else {
                RawTypeSyntaxKind::Named {
                    name: RawIdentifierSyntax { text: "i32".to_owned(), span: element_span },
                }
            },
        });
        let vec_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: vec_span,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element_id,
                greater_than_span,
            },
        });
        vec_type_ids.push(vec_id);
    }
    let scalar_span = token("String", usize::from(element == "String") * 5);
    let scalar_ty = u32::try_from(types.len()).expect("type id");
    types.push(RawTypeSyntax {
        span: scalar_span,
        kind: RawTypeSyntaxKind::String { keyword_span: scalar_span },
    });
    let root_span = range("{", 0, "}", 2);
    let then_span = range("{", 1, "}", 0);
    let else_span = range("{", 2, "}", 1);
    let target_span = if push_outer { token("own", 1) } else { token("branch", 1) };
    let expressions = vec![
        RawExpressionSyntax {
            span: range(&vec_spelling, 2, ")", 1),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_type_ids[2],
                open_paren_span: token("(", 1),
                open_bracket_span: token("[", 0),
                elements: Vec::new(),
                close_bracket_span: token("]", 0),
                close_paren_span: token(")", 1),
            },
        },
        RawExpressionSyntax {
            span: token("flag", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
            },
        },
        RawExpressionSyntax {
            span: range(&vec_spelling, 4, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_type_ids[4],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: Vec::new(),
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: target_span,
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: target.to_owned(), span: target_span },
            },
        },
        RawExpressionSyntax {
            span: token(pushed, 0),
            kind: if element == "String" {
                RawExpressionKind::StringLiteral { spelling: pushed.to_owned() }
            } else {
                RawExpressionKind::I32Literal { spelling: pushed.to_owned() }
            },
        },
        RawExpressionSyntax {
            span: range("push", 0, ")", 4),
            kind: RawExpressionKind::VecPush {
                keyword_span: token("push", 0),
                open_paren_span: token("(", 4),
                vector: 3,
                comma_span: token(",", 0),
                value: 4,
                close_paren_span: token(")", 4),
            },
        },
        RawExpressionSyntax {
            span: token("\"e\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"e\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("own", usize::from(push_outer) + 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "own".to_owned(),
                    span: token("own", usize::from(push_outer) + 1),
                },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("const own", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
                name: RawIdentifierSyntax { text: "own".to_owned(), span: token("own", 0) },
                type_syntax: vec_type_ids[1],
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("if", 0, "}", 1),
            kind: RawStatementKind::If {
                keyword_span: token("if", 0),
                open_paren_span: token("(", 2),
                condition: 1,
                close_paren_span: token(")", 2),
                then_block: 1,
                else_clause: Some(RawElseSyntax { keyword_span: token("else", 0), block: 2 }),
            },
        },
        RawStatementSyntax {
            span: range("let branch", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("let", 0),
                mutable: true,
                name: RawIdentifierSyntax { text: "branch".to_owned(), span: token("branch", 0) },
                type_syntax: vec_type_ids[3],
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("push", 0, ";", 2),
            kind: RawStatementKind::ExpressionStatement {
                expression: 5,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("const value", 0, ";", 3),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "value".to_owned(), span: token("value", 0) },
                type_syntax: scalar_ty,
                equals_span: token("=", 2),
                initializer: 6,
                semicolon_span: token(";", 3),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 4),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 7,
                semicolon_span: token(";", 4),
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
        result_type: vec_type_ids[0],
        body: RawFunctionBodySyntax {
            span: root_span,
            root_block: 0,
            blocks: vec![
                RawBlockSyntax {
                    span: root_span,
                    open_brace_span: token("{", 0),
                    statements: vec![0, 1, 5],
                    close_brace_span: token("}", 2),
                },
                RawBlockSyntax {
                    span: then_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
                    close_brace_span: token("}", 0),
                },
                RawBlockSyntax {
                    span: else_span,
                    open_brace_span: token("{", 2),
                    statements: vec![4],
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
fn private_vec_if_restores_outer_owner_and_drops_branch_vec() {
    let (source, raw) = private_vec_if_fixture(false, "i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned Vec if");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Vec if must verify");
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
    assert_eq!(
        blocks[1]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecPush,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let then_drop = blocks[1]
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .expect("branch Vec drop");
    assert_eq!(
        then_drop
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![4]
    );
    assert!(function.cleanup_plans().all(|plan| {
        plan.actions().all(|place| place.index() != 4)
            || plan.site().role() == VerifiedCleanupRole::PrepareFailure
    }));
}

#[test]
fn private_vec_if_rejects_push_into_incoming_vec_before_rhs() {
    let (source, raw) = private_vec_if_fixture(true, "i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful outer Vec push");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("outer Vec push must reject");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3015")
        .expect("join-safety diagnostic");
    assert_eq!(
        diagnostic.primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "own", 1)))
    );
}

#[test]
fn private_vec_string_if_constructs_pushes_and_drops_branch_owner_once() {
    let (source, raw) = private_vec_if_fixture(false, "String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> if");
    let program = lower(pair_input(&syntax, &sources)).expect("Vec<String> branch must verify");
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
        blocks[1]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecPush,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    assert_eq!(
        blocks[1]
            .instructions()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
            .count(),
        1
    );
    assert_eq!(
        blocks[2]
            .instructions()
            .filter(|instruction| { instruction.kind() == VerifiedInstructionKind::DropPlace })
            .count(),
        1
    );
}
