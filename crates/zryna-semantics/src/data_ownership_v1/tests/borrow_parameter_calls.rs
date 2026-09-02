use super::*;

fn named_type(source: &str, spelling: &str, ordinal: usize) -> RawTypeSyntax {
    let span = nth_untrusted_span(source, spelling, ordinal);
    RawTypeSyntax {
        span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: spelling.to_owned(), span },
        },
    }
}

fn borrow_type(
    source: &str,
    spelling: &str,
    ordinal: usize,
    argument: u32,
    exclusive: bool,
) -> RawTypeSyntax {
    let span = nth_untrusted_span(source, spelling, ordinal);
    let keyword = if exclusive { "BorrowMut" } else { "Borrow" };
    let keyword_span = zryna_source::UntrustedSpan {
        file: 0,
        start: span.start,
        end: span.start + u32::try_from(keyword.len()).expect("keyword length"),
    };
    let less_than_span =
        zryna_source::UntrustedSpan { file: 0, start: keyword_span.end, end: keyword_span.end + 1 };
    let greater_than_span =
        zryna_source::UntrustedSpan { file: 0, start: span.end - 1, end: span.end };
    let kind = if exclusive {
        RawTypeSyntaxKind::BorrowMut { keyword_span, less_than_span, argument, greater_than_span }
    } else {
        RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
    };
    RawTypeSyntax { span, kind }
}

fn parameter(
    source: &str,
    name: &str,
    spelling: &str,
    ordinal: usize,
    type_syntax: u32,
) -> RawParameterSyntax {
    let span = nth_untrusted_span(source, &format!("{name}: {spelling}"), ordinal);
    let name_span = zryna_source::UntrustedSpan {
        file: 0,
        start: span.start,
        end: span.start + u32::try_from(name.len()).expect("name length"),
    };
    RawParameterSyntax {
        span,
        name: RawIdentifierSyntax { text: name.to_owned(), span: name_span },
        type_syntax,
    }
}

fn reference(name: &str, span: zryna_source::UntrustedSpan) -> RawExpressionSyntax {
    RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: name.to_owned(), span },
        },
    }
}

fn mixed_source(write_target: &str, call_arguments: &[&str; 4], first_exclusive: bool) -> String {
    let first_borrow = if first_exclusive { "BorrowMut<i32>" } else { "Borrow<i32>" };
    format!(
        "function relay(left: i32, shared: {first_borrow}, right: bool, exclusive: BorrowMut<i32>): i32 {{ {write_target} = left; return shared; }} function caller(left: i32, shared: {first_borrow}, right: bool, exclusive: BorrowMut<i32>): i32 {{ return relay({}); }}",
        call_arguments.join(", ")
    )
}

fn relay_body(source: &str, write_target: &str) -> RawFunctionBodySyntax {
    let assignment_text = format!("{write_target} = left;");
    let assignment = nth_untrusted_span(source, &assignment_text, 0);
    let target_span = zryna_source::UntrustedSpan {
        file: 0,
        start: assignment.start,
        end: assignment.start + u32::try_from(write_target.len()).expect("target length"),
    };
    let equals_start = target_span.end + 1;
    let value_span =
        zryna_source::UntrustedSpan { file: 0, start: equals_start + 2, end: equals_start + 6 };
    let return_statement = nth_untrusted_span(source, "return shared;", 0);
    let returned_span = zryna_source::UntrustedSpan {
        file: 0,
        start: return_statement.start + 7,
        end: return_statement.end - 1,
    };
    let body_text = format!("{{ {assignment_text} return shared; }}");
    let body = nth_untrusted_span(source, &body_text, 0);
    RawFunctionBodySyntax {
        span: body,
        root_block: 0,
        blocks: vec![RawBlockSyntax {
            span: body,
            open_brace_span: zryna_source::UntrustedSpan {
                file: 0,
                start: body.start,
                end: body.start + 1,
            },
            statements: vec![0, 1],
            close_brace_span: zryna_source::UntrustedSpan {
                file: 0,
                start: body.end - 1,
                end: body.end,
            },
        }],
        statements: vec![
            RawStatementSyntax {
                span: assignment,
                kind: RawStatementKind::Assignment {
                    target: 0,
                    equals_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: equals_start,
                        end: equals_start + 1,
                    },
                    value: 1,
                    semicolon_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: assignment.end - 1,
                        end: assignment.end,
                    },
                },
            },
            RawStatementSyntax {
                span: return_statement,
                kind: RawStatementKind::Return {
                    keyword_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: return_statement.start,
                        end: return_statement.start + 6,
                    },
                    value: 2,
                    semicolon_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: return_statement.end - 1,
                        end: return_statement.end,
                    },
                },
            },
        ],
        expressions: vec![
            reference(write_target, target_span),
            reference("left", value_span),
            reference("shared", returned_span),
        ],
    }
}

fn caller_body(source: &str, call_arguments: &[&str; 4]) -> RawFunctionBodySyntax {
    let call_text = format!("relay({})", call_arguments.join(", "));
    let call = nth_untrusted_span(source, &call_text, 0);
    let statement = nth_untrusted_span(source, &format!("return {call_text};"), 0);
    let body = nth_untrusted_span(source, &format!("{{ return {call_text}; }}"), 0);
    let mut expressions = Vec::with_capacity(5);
    let mut start = call.start + 6;
    for argument in call_arguments {
        let end = start + u32::try_from(argument.len()).expect("argument length");
        expressions.push(reference(argument, zryna_source::UntrustedSpan { file: 0, start, end }));
        start = end + 2;
    }
    expressions.push(RawExpressionSyntax {
        span: call,
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "relay".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: call.start,
                    end: call.start + 5,
                },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: call.start + 5,
                end: call.start + 6,
            },
            arguments: vec![0, 1, 2, 3],
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: call.end - 1,
                end: call.end,
            },
        },
    });
    RawFunctionBodySyntax {
        span: body,
        root_block: 0,
        blocks: vec![RawBlockSyntax {
            span: body,
            open_brace_span: zryna_source::UntrustedSpan {
                file: 0,
                start: body.start,
                end: body.start + 1,
            },
            statements: vec![0],
            close_brace_span: zryna_source::UntrustedSpan {
                file: 0,
                start: body.end - 1,
                end: body.end,
            },
        }],
        statements: vec![RawStatementSyntax {
            span: statement,
            kind: RawStatementKind::Return {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: statement.start,
                    end: statement.start + 6,
                },
                value: 4,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: statement.end - 1,
                    end: statement.end,
                },
            },
        }],
        expressions,
    }
}

fn mixed_snapshot(
    source: &str,
    write_target: &str,
    call_arguments: &[&str; 4],
    first_exclusive: bool,
) -> RawProjectSyntaxSnapshot {
    let first_borrow = if first_exclusive { "BorrowMut<i32>" } else { "Borrow<i32>" };
    let type_syntax = vec![
        named_type(source, "i32", 0),
        named_type(source, "i32", 1),
        borrow_type(source, first_borrow, 0, 1, first_exclusive),
        named_type(source, "bool", 0),
        named_type(source, "i32", 2),
        borrow_type(source, "BorrowMut<i32>", usize::from(first_exclusive), 4, true),
        named_type(source, "i32", 3),
        named_type(source, "i32", 4),
        named_type(source, "i32", 5),
        borrow_type(source, first_borrow, if first_exclusive { 2 } else { 1 }, 8, first_exclusive),
        named_type(source, "bool", 1),
        named_type(source, "i32", 6),
        borrow_type(source, "BorrowMut<i32>", if first_exclusive { 3 } else { 1 }, 11, true),
        named_type(source, "i32", 7),
    ];
    let parameters = |ordinal, offset| {
        vec![
            parameter(source, "left", "i32", ordinal, offset),
            parameter(source, "shared", first_borrow, ordinal, offset + 2),
            parameter(source, "right", "bool", ordinal, offset + 3),
            parameter(source, "exclusive", "BorrowMut<i32>", ordinal, offset + 5),
        ]
    };
    let relay_body = relay_body(source, write_target);
    let caller_body = caller_body(source, call_arguments);
    let relay_function = nth_untrusted_span(source, "function", 0);
    let caller_function = nth_untrusted_span(source, "function", 1);
    let relay_name = nth_untrusted_span(source, "relay", 0);
    let caller_name = nth_untrusted_span(source, "caller", 0);
    RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax,
            data_declarations: Vec::new(),
            functions: vec![
                RawFunctionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: relay_function.start,
                        end: relay_body.span.end,
                    },
                    export_span: None,
                    function_span: relay_function,
                    name: RawIdentifierSyntax { text: "relay".to_owned(), span: relay_name },
                    parameters: parameters(0, 0),
                    result_type: 6,
                    body: relay_body,
                },
                RawFunctionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: caller_function.start,
                        end: caller_body.span.end,
                    },
                    export_span: None,
                    function_span: caller_function,
                    name: RawIdentifierSyntax { text: "caller".to_owned(), span: caller_name },
                    parameters: parameters(1, 7),
                    result_type: 13,
                    body: caller_body,
                },
            ],
        }],
        diagnostics: Vec::new(),
    }
}

#[test]
fn mixed_value_shared_value_exclusive_parameters_lower_in_canonical_ir_order() {
    let arguments = ["left", "shared", "right", "exclusive"];
    let source = mixed_source("exclusive", &arguments, false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(mixed_snapshot(&source, "exclusive", &arguments, false), &sources)
        .expect("source-faithful mixed borrow call");
    let program = lower(pair_input(&syntax, &sources)).expect("mixed borrow call lowering");
    let module = program.modules().next().expect("module");
    let mut functions = module.functions();
    let relay = functions.next().expect("relay");
    let caller = functions.next().expect("caller");

    assert_eq!(relay.parameters().map(|value| value.id().index()).collect::<Vec<_>>(), [0, 1]);
    let borrow_parameters = relay.borrow_parameters().collect::<Vec<_>>();
    assert_eq!(
        borrow_parameters.iter().map(|borrow| borrow.id().index()).collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(borrow_parameters[0].access(), VerifiedBorrowAccess::Shared);
    assert_eq!(borrow_parameters[1].access(), VerifiedBorrowAccess::Exclusive);
    assert_eq!(
        relay.places().map(zryna_ir::data_ownership_v1::VerifiedPlace::kind).collect::<Vec<_>>(),
        [VerifiedPlaceKind::Parameter(0), VerifiedPlaceKind::Parameter(1)]
    );
    let relay_kinds = relay
        .blocks()
        .next()
        .expect("relay block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        relay_kinds,
        [
            VerifiedInstructionKind::CopyFromPlace,
            VerifiedInstructionKind::BorrowWrite,
            VerifiedInstructionKind::BorrowRead,
        ]
    );
    assert!(!relay_kinds.contains(&VerifiedInstructionKind::BeginBorrow));
    assert!(!relay_kinds.contains(&VerifiedInstructionKind::EndBorrow));

    let call = caller
        .blocks()
        .next()
        .expect("caller block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        .expect("DirectCall");
    let arguments = call.call_arguments().collect::<Vec<_>>();
    assert!(matches!(arguments[0], VerifiedCallArgument::Value(value) if value.index() == 2));
    assert!(matches!(arguments[1], VerifiedCallArgument::Value(value) if value.index() == 3));
    assert!(matches!(arguments[2], VerifiedCallArgument::Borrow(borrow) if borrow.index() == 0));
    assert!(matches!(arguments[3], VerifiedCallArgument::Borrow(borrow) if borrow.index() == 1));
}

#[test]
fn shared_borrow_parameter_write_is_rejected_before_rhs_lowering() {
    let arguments = ["left", "shared", "right", "exclusive"];
    let source = mixed_source("shared", &arguments, false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(mixed_snapshot(&source, "shared", &arguments, false), &sources)
        .expect("source-faithful shared write");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("shared write");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].message(), "shared borrow parameters are read-only");
    let assignment = nth_untrusted_span(&source, "shared = left", 0);
    let target =
        zryna_source::UntrustedSpan { file: 0, start: assignment.start, end: assignment.start + 6 };
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(sources.verify_span(target).expect("shared target span"))
    );
}

#[test]
fn borrow_call_rejects_access_mismatch_and_nonborrow_arguments() {
    for (arguments, message) in [
        (
            ["left", "exclusive", "right", "shared"],
            "borrow argument does not match the callee referent and access",
        ),
        (
            ["left", "left", "right", "exclusive"],
            "borrow arguments must forward an in-scope borrow parameter",
        ),
    ] {
        let source = mixed_source("exclusive", &arguments, false);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(mixed_snapshot(&source, "exclusive", &arguments, false), &sources)
                .expect("source-faithful rejected borrow call");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("rejected borrow call");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
        assert_eq!(diagnostics[0].message(), message);
    }
}

#[test]
fn borrow_call_rejects_reused_exclusive_authority() {
    let arguments = ["left", "shared", "right", "shared"];
    let source = mixed_source("exclusive", &arguments, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(mixed_snapshot(&source, "exclusive", &arguments, true), &sources)
        .expect("source-faithful duplicate exclusive call");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("duplicate exclusive call");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].message(),
        "exclusive borrow arguments cannot reuse the same authority"
    );
}
