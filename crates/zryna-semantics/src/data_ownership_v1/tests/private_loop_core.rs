use super::*;

#[allow(clippy::too_many_lines)]
fn private_vec_push_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_vec_push_loop_fixture_with_mutability(true)
}

#[allow(clippy::too_many_lines)]
fn private_vec_push_loop_fixture_with_mutability(
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let declaration = if mutable { "let" } else { "const" };
    let text = format!(
        "function keep(flag: bool): Vec<i32> {{ {declaration} outer: Vec<i32> = Vec<i32>([]); while (flag) {{ push(outer, 1); }} return outer; }}"
    );
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
    let one_span = token("1", 0);
    let expressions = vec![
        RawExpressionSyntax {
            span: range("Vec<i32>", 2, ")", 1),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[2],
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
            span: token("outer", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
            },
        },
        RawExpressionSyntax {
            span: one_span,
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("push", 0, ")", 3),
            kind: RawExpressionKind::VecPush {
                keyword_span: token("push", 0),
                open_paren_span: token("(", 3),
                vector: 2,
                comma_span: token(",", 0),
                value: 3,
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: token("outer", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 2) },
            },
        },
    ];
    let declaration_start = format!("{declaration} outer");
    let statements = vec![
        RawStatementSyntax {
            span: range(&declaration_start, 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token(declaration, 0),
                mutable,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: vec_types[1],
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 2),
                condition: 1,
                close_paren_span: token(")", 2),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("push", 0, ";", 1),
            kind: RawStatementKind::ExpressionStatement {
                expression: 4,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 2),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 5,
                semicolon_span: token(";", 2),
            },
        },
    ];
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let function = RawFunctionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(text.len()).expect("fixture length"),
        },
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "keep".to_owned(), span: token("keep", 0) },
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
                    statements: vec![0, 1, 3],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2],
                    close_brace_span: token("}", 0),
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
fn private_string_loop_restores_incoming_owner_and_reverse_drops_body_locals() {
    let (source, raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private String loop");
    let program = lower(pair_input(&syntax, &sources)).expect("private String loop must verify");
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
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let entry = blocks[0].terminator().edges().next().expect("preheader edge");
    assert_eq!(entry.target().index(), 1);
    assert_eq!(entry.arguments().count(), 0);
    let header = blocks[1].terminator();
    assert_eq!(header.value_operands().next().expect("loop condition").index(), 2);
    let (body, exit) = header.branch_edges().expect("header branch");
    assert_eq!((body.target().index(), exit.target().index()), (2, 3));
    assert_eq!((body.arguments().count(), exit.arguments().count()), (0, 0));
    let backedge = blocks[2].terminator().edges().next().expect("loop backedge");
    assert_eq!(backedge.target().index(), 1);
    assert_eq!(backedge.arguments().count(), 0);
    let body_kinds = blocks[2]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        body_kinds,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let dropped = blocks[2]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(dropped, vec![6, 4]);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("returned owner").index(), 5);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_loop_restores_incoming_owner_and_reverse_drops_body_locals() {
    let (source, raw) = private_vec_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful private Vec loop");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec loop must verify");
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
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let entry = blocks[0].terminator().edges().next().expect("preheader edge");
    assert_eq!(entry.target().index(), 1);
    assert_eq!(entry.arguments().count(), 0);
    let header = blocks[1].terminator();
    assert_eq!(header.value_operands().next().expect("loop condition").index(), 2);
    let (body, exit) = header.branch_edges().expect("header branch");
    assert_eq!((body.target().index(), exit.target().index()), (2, 3));
    assert_eq!((body.arguments().count(), exit.arguments().count()), (0, 0));
    let backedge = blocks[2].terminator().edges().next().expect("loop backedge");
    assert_eq!(backedge.target().index(), 1);
    assert_eq!(backedge.arguments().count(), 0);
    let body_kinds = blocks[2]
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        body_kinds,
        vec![
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let dropped = blocks[2]
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::DropPlace)
        .flat_map(zryna_ir::data_ownership_v1::VerifiedInstruction::place_operands)
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(dropped, vec![6, 4]);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("returned owner").index(), 7);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_loop_pushes_into_one_stable_outer_place_with_failure_cleanup() {
    let (source, raw) = private_vec_push_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec push loop");
    let program = lower(pair_input(&syntax, &sources)).expect("Vec push loop must verify");
    let replay = lower(pair_input(&syntax, &sources)).expect("Vec push replay must verify");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
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
    assert!(blocks.iter().all(|block| block.parameters().count() == 0));
    assert_eq!(blocks[1].terminator().value_operands().next().expect("condition").index(), 2);
    assert_eq!(
        blocks[2]
            .instructions()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::I32Literal, VerifiedInstructionKind::VecPush]
    );
    let push = blocks[2].instructions().nth(1).expect("VecPush commit");
    assert_eq!(
        push.place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let cleanup = push.cleanup().expect("VecPush failure cleanup");
    let actions = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .expect("VecPush cleanup plan")
        .actions()
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(actions, vec![2]);
    assert_eq!(blocks[2].terminator().edges().next().expect("backedge").target().index(), 1);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("return").index(), 4);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_vec_mutation_loop_rejects_immutable_target_at_exact_reference() {
    let (source, raw) = private_vec_push_loop_fixture_with_mutability(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable Vec loop");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable Vec must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    let target = nth_untrusted_span(&source, "outer", 1);
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, target)));
}
