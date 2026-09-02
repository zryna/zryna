use super::*;

#[allow(clippy::too_many_lines)]
fn private_string_mutation_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::Literal)
}

#[derive(Clone, Copy)]
pub(super) enum StringLoopReplacement {
    Literal,
    Move,
    Call,
    CloneRead,
    ConcatRead,
    CloneCall,
    ConcatCall,
}

#[allow(clippy::too_many_lines)]
pub(super) fn private_string_mutation_loop_fixture_with_options(
    mutable: bool,
    replacement: StringLoopReplacement,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let declaration = if mutable { "let" } else { "const" };
    let replacement_source = match replacement {
        StringLoopReplacement::Literal => "\"after\"",
        StringLoopReplacement::Move => "outer",
        StringLoopReplacement::Call => "take(outer)",
        StringLoopReplacement::CloneRead => "clone(outer)",
        StringLoopReplacement::ConcatRead => "concat(outer, \"x\")",
        StringLoopReplacement::CloneCall => "clone(take(outer))",
        StringLoopReplacement::ConcatCall => "concat(take(outer), \"x\")",
    };
    let text = format!(
        "function keep(flag: bool): String {{ {declaration} outer: String = \"before\"; while (flag) {{ outer = {replacement_source}; }} return outer; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..2).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
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
    let mut expressions = vec![
        RawExpressionSyntax {
            span: token("\"before\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"before\"".to_owned() },
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
    ];
    let mut push_outer_reference = || {
        let id = u32::try_from(expressions.len()).expect("outer expression id");
        expressions.push(RawExpressionSyntax {
            span: token("outer", 2),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 2) },
            },
        });
        id
    };
    let replacement_id = match replacement {
        StringLoopReplacement::Literal => {
            let id = u32::try_from(expressions.len()).expect("literal expression id");
            expressions.push(RawExpressionSyntax {
                span: token("\"after\"", 0),
                kind: RawExpressionKind::StringLiteral { spelling: "\"after\"".to_owned() },
            });
            id
        }
        StringLoopReplacement::Move => push_outer_reference(),
        StringLoopReplacement::Call
        | StringLoopReplacement::CloneRead
        | StringLoopReplacement::ConcatRead
        | StringLoopReplacement::CloneCall
        | StringLoopReplacement::ConcatCall => {
            let outer = push_outer_reference();
            let consumed = matches!(
                replacement,
                StringLoopReplacement::Call
                    | StringLoopReplacement::CloneCall
                    | StringLoopReplacement::ConcatCall
            );
            let operand = if consumed {
                let id = u32::try_from(expressions.len()).expect("call expression id");
                let (open, close) = if matches!(replacement, StringLoopReplacement::Call) {
                    (2, 2)
                } else {
                    (3, 2)
                };
                expressions.push(RawExpressionSyntax {
                    span: range("take", 0, ")", close),
                    kind: RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "take".to_owned(),
                            span: token("take", 0),
                        },
                        open_paren_span: token("(", open),
                        arguments: vec![outer],
                        close_paren_span: token(")", close),
                    },
                });
                id
            } else {
                outer
            };
            if matches!(replacement, StringLoopReplacement::Call) {
                operand
            } else if matches!(
                replacement,
                StringLoopReplacement::CloneRead | StringLoopReplacement::CloneCall
            ) {
                let id = u32::try_from(expressions.len()).expect("clone expression id");
                let close = if consumed { 3 } else { 2 };
                expressions.push(RawExpressionSyntax {
                    span: range("clone", 0, ")", close),
                    kind: RawExpressionKind::Clone {
                        keyword_span: token("clone", 0),
                        open_paren_span: token("(", 2),
                        value: operand,
                        close_paren_span: token(")", close),
                    },
                });
                id
            } else {
                let literal = u32::try_from(expressions.len()).expect("literal expression id");
                expressions.push(RawExpressionSyntax {
                    span: token("\"x\"", 0),
                    kind: RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
                });
                let id = u32::try_from(expressions.len()).expect("concat expression id");
                let close = if consumed { 3 } else { 2 };
                expressions.push(RawExpressionSyntax {
                    span: range("concat", 0, ")", close),
                    kind: RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "concat".to_owned(),
                            span: token("concat", 0),
                        },
                        open_paren_span: token("(", 2),
                        arguments: vec![operand, literal],
                        close_paren_span: token(")", close),
                    },
                });
                id
            }
        }
    };
    let return_id = u32::try_from(expressions.len()).expect("return expression id");
    let return_ordinal = usize::from(!matches!(replacement, StringLoopReplacement::Literal)) + 2;
    expressions.push(RawExpressionSyntax {
        span: token("outer", return_ordinal),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "outer".to_owned(),
                span: token("outer", return_ordinal),
            },
        },
    });
    let declaration_start = format!("{declaration} outer");
    let statements = vec![
        RawStatementSyntax {
            span: range(&declaration_start, 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token(declaration, 0),
                mutable,
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 0) },
                type_syntax: 2,
                equals_span: token("=", 0),
                initializer: 0,
                semicolon_span: token(";", 0),
            },
        },
        RawStatementSyntax {
            span: range("while", 0, "}", 0),
            kind: RawStatementKind::While {
                keyword_span: token("while", 0),
                open_paren_span: token("(", 1),
                condition: 1,
                close_paren_span: token(")", 1),
                body_block: 1,
            },
        },
        RawStatementSyntax {
            span: range("outer", 1, ";", 1),
            kind: RawStatementKind::Assignment {
                target: 2,
                equals_span: token("=", 1),
                value: replacement_id,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 2),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: return_id,
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
        result_type: 1,
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
fn private_string_mutation_loop_rejects_immutable_and_self_move_before_rhs() {
    for (mutable, replacement) in
        [(false, StringLoopReplacement::Literal), (true, StringLoopReplacement::Move)]
    {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(mutable, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful mutation negative");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("mutation must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        let target = nth_untrusted_span(
            &source,
            "outer",
            if matches!(replacement, StringLoopReplacement::Move) { 2 } else { 1 },
        );
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, target)));
    }

    let (source, raw) =
        private_string_mutation_loop_fixture_with_options(true, StringLoopReplacement::Call);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming-call negative");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("incoming call move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
    let argument = nth_untrusted_span(&source, "outer", 2);
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, argument)));
}

#[test]
fn private_string_mutation_loop_finds_nested_consumers_but_allows_direct_reads() {
    for replacement in [StringLoopReplacement::CloneCall, StringLoopReplacement::ConcatCall] {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(true, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested consumer");
        let diagnostics =
            lower(pair_input(&syntax, &sources)).expect_err("nested move must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
        let inner = nth_untrusted_span(&source, "outer", 2);
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, inner)));
    }

    for replacement in [StringLoopReplacement::CloneRead, StringLoopReplacement::ConcatRead] {
        let (source, raw) = private_string_mutation_loop_fixture_with_options(true, replacement);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful direct read");
        let program = lower(pair_input(&syntax, &sources))
            .expect("direct clone/concat read must remain admitted");
        if matches!(replacement, StringLoopReplacement::ConcatRead) {
            let function = program
                .verified_ir()
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("function");
            let body = function.blocks().nth(2).expect("loop body");
            let last = body.instructions().last().expect("temporary read drop");
            assert_eq!(last.kind(), VerifiedInstructionKind::DropPlace);
            assert_eq!(last.place_operands().next().expect("dropped literal").index(), 3);
        }
    }
}
#[test]
fn private_string_loop_replaces_one_stable_outer_place_with_failure_cleanup() {
    let (source, raw) = private_string_mutation_loop_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String mutation loop");
    let program = lower(pair_input(&syntax, &sources)).expect("String mutation loop must verify");
    let replay = lower(pair_input(&syntax, &sources)).expect("String mutation replay must verify");
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
        vec![VerifiedInstructionKind::StringFromUtf8, VerifiedInstructionKind::ReplacePlace]
    );
    let prepare = blocks[2].instructions().next().expect("replacement prepare");
    let cleanup = prepare.cleanup().expect("prepare failure cleanup");
    let actions = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .expect("prepare cleanup plan")
        .actions()
        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(actions, vec![2]);
    let replacement = blocks[2].instructions().nth(1).expect("replacement commit");
    assert_eq!(
        replacement
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(blocks[2].terminator().edges().next().expect("backedge").target().index(), 1);
    assert_eq!(blocks[3].terminator().value_operands().next().expect("return").index(), 4);
    assert_eq!(blocks[3].terminator().derived_drop_actions().count(), 0);
}
