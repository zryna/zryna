use super::*;

#[derive(Clone, Copy)]
enum StringAssignmentRhs {
    Move,
    Literal,
    Clone,
    Concat,
    SelfMove,
    CallSelf,
    CloneCallSelf,
}

#[allow(clippy::too_many_lines)]
fn string_assignment_snapshot(
    rhs: StringAssignmentRhs,
) -> (&'static str, RawProjectSyntaxSnapshot) {
    const LITERAL: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = \"fresh\"; return x; }";
    const CLONE: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = clone(y); return x; }";
    const CONCAT: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = concat(x, y); return x; }";
    const SELF_MOVE: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = x; return x; }";
    const CALL_SELF: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = take(x); return x; }";
    const CLONE_CALL_SELF: &str = "function assign(): String { let x: String = \"old\"; const y: String = \"new\"; x = clone(take(x)); return x; }";
    let mut raw = response_snapshot(STRING_ASSIGN_MOVE_RESPONSE);
    let (source, extra) = match rhs {
        StringAssignmentRhs::Move => return (STRING_ASSIGN_MOVE_SOURCE, raw),
        StringAssignmentRhs::Literal => (LITERAL, 6),
        StringAssignmentRhs::Clone => (CLONE, 7),
        StringAssignmentRhs::Concat => (CONCAT, 11),
        StringAssignmentRhs::SelfMove => {
            let zryna_syntax::v4::RawExpressionKind::Reference { name } =
                &mut raw.files[0].functions[0].body.expressions[3].kind
            else {
                panic!("assignment source")
            };
            name.text = "x".to_owned();
            return (SELF_MOVE, raw);
        }
        StringAssignmentRhs::CallSelf => (CALL_SELF, 6),
        StringAssignmentRhs::CloneCallSelf => (CLONE_CALL_SELF, 13),
    };
    raw = shift_snapshot(raw, 83, extra);
    let body = &mut raw.files[0].functions[0].body;
    body.statements[2].span.end += extra;
    let RawStatementKind::Assignment { value, semicolon_span, .. } = &mut body.statements[2].kind
    else {
        panic!("assignment")
    };
    semicolon_span.start += extra;
    semicolon_span.end += extra;
    match rhs {
        StringAssignmentRhs::Literal => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 87 },
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"fresh\"".to_owned(),
                },
            };
        }
        StringAssignmentRhs::Clone => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "y".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                    },
                },
            };
            *value = u32::try_from(body.expressions.len()).expect("expression id");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 88 },
                kind: zryna_syntax::v4::RawExpressionKind::Clone {
                    keyword_span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 85 },
                    open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 85, end: 86 },
                    value: 3,
                    close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                },
            });
        }
        StringAssignmentRhs::Concat => {
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 87, end: 88 },
                    },
                },
            };
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 90, end: 91 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "y".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 90, end: 91 },
                    },
                },
            });
            *value = u32::try_from(body.expressions.len()).expect("expression id");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 92 },
                kind: zryna_syntax::v4::RawExpressionKind::Call {
                    callee: RawIdentifierSyntax {
                        text: "concat".to_owned(),
                        span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 86 },
                    },
                    open_paren_span: zryna_source::UntrustedSpan { file: 0, start: 86, end: 87 },
                    arguments: vec![3, 5],
                    close_paren_span: zryna_source::UntrustedSpan { file: 0, start: 91, end: 92 },
                },
            });
        }
        StringAssignmentRhs::CallSelf | StringAssignmentRhs::CloneCallSelf => {
            let nested = matches!(rhs, StringAssignmentRhs::CloneCallSelf);
            let reference = if nested { (91, 92) } else { (85, 86) };
            body.expressions[3] = RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: reference.0, end: reference.1 },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: reference.0,
                            end: reference.1,
                        },
                    },
                },
            };
            let call_id = u32::try_from(body.expressions.len()).expect("call id");
            let (call_start, call_end, open, close) =
                if nested { (86, 93, 90, 92) } else { (80, 87, 84, 86) };
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: call_start, end: call_end },
                kind: zryna_syntax::v4::RawExpressionKind::Call {
                    callee: RawIdentifierSyntax {
                        text: "take".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: call_start,
                            end: call_start + 4,
                        },
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: open,
                        end: open + 1,
                    },
                    arguments: vec![3],
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: close,
                        end: close + 1,
                    },
                },
            });
            if nested {
                *value = u32::try_from(body.expressions.len()).expect("clone id");
                body.expressions.push(RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 94 },
                    kind: zryna_syntax::v4::RawExpressionKind::Clone {
                        keyword_span: zryna_source::UntrustedSpan { file: 0, start: 80, end: 85 },
                        open_paren_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: 85,
                            end: 86,
                        },
                        value: call_id,
                        close_paren_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: 93,
                            end: 94,
                        },
                    },
                });
            } else {
                *value = call_id;
            }
        }
        StringAssignmentRhs::Move | StringAssignmentRhs::SelfMove => unreachable!(),
    }
    (source, raw)
}

#[test]
fn private_string_root_assignment_prepares_then_replaces_exact_owner() {
    for rhs in [
        StringAssignmentRhs::Move,
        StringAssignmentRhs::Literal,
        StringAssignmentRhs::Clone,
        StringAssignmentRhs::Concat,
    ] {
        let (source, raw) = string_assignment_snapshot(rhs);
        let sources = sources_for(source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful String assignment v4");
        let program = lower(pair_input(&syntax, &sources)).expect("private String assignment");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let replace = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
            .expect("ReplacePlace");
        let actions = replace.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].root().index(), 1);
        assert_eq!(actions[0].moved_projections().count(), 0);
        assert_eq!(actions[0].initialized_projections().count(), 0);
        assert_eq!(actions[0].active_variant(), None);
        let prepare = block
            .instructions()
            .filter(|instruction| {
                matches!(
                    instruction.kind(),
                    VerifiedInstructionKind::StringFromUtf8
                        | VerifiedInstructionKind::StringClone
                        | VerifiedInstructionKind::StringConcat
                ) && instruction.derived_drop_actions().any(|action| action.root().index() == 1)
            })
            .last();
        if matches!(
            rhs,
            StringAssignmentRhs::Literal | StringAssignmentRhs::Clone | StringAssignmentRhs::Concat
        ) {
            assert!(prepare.is_some(), "fallible RHS retains the old destination");
        }
        let return_roots = block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>();
        if matches!(rhs, StringAssignmentRhs::Move) {
            assert!(return_roots.is_empty());
        } else {
            assert_eq!(return_roots, [3]);
        }
    }
}

#[test]
fn private_string_self_assignment_move_is_narrow_and_deterministic() {
    let (source, raw) = string_assignment_snapshot(StringAssignmentRhs::SelfMove);
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful self assignment v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("self move assignment");
    let second = lower(pair_input(&syntax, &sources)).expect_err("same self move assignment");
    let summarize = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_owned(),
                    diagnostic.primary_span().map(|span| (span.start(), span.end())),
                    diagnostic.message().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(summarize(&first), summarize(&second));
    assert_eq!(first[0].code(), "ZRYNA-M3014");
    let at = first[0].primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (80, 81));
}

#[test]
fn private_string_assignment_rejects_call_based_target_consumption_before_rhs() {
    for rhs in [StringAssignmentRhs::CallSelf, StringAssignmentRhs::CloneCallSelf] {
        let (source, raw) = string_assignment_snapshot(rhs);
        let sources = sources_for(source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful consuming assignment");
        let diagnostics =
            lower(pair_input(&syntax, &sources)).expect_err("target move must reject");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
        let expected = nth_untrusted_span(source, "x", 2);
        assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, expected)));
    }
}

#[test]
fn private_string_assignment_rejects_an_immutable_target() {
    let source = STRING_ASSIGN_MOVE_SOURCE.replacen("let x", "const x", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(STRING_ASSIGN_MOVE_RESPONSE), 32, 2);
    let RawStatementKind::LocalDeclaration {
        keyword_span,
        name,
        type_syntax,
        equals_span,
        initializer,
        semicolon_span,
        ..
    } = raw.files[0].functions[0].body.statements[0].kind.clone()
    else {
        panic!("first local")
    };
    let mut keyword_span = keyword_span;
    keyword_span.end = 33;
    raw.files[0].functions[0].body.statements[0].kind = RawStatementKind::LocalDeclaration {
        keyword_span,
        mutable: false,
        name,
        type_syntax,
        equals_span,
        initializer,
        semicolon_span,
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful immutable assignment");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("immutable assignment");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3014")
        .expect("immutable target diagnostic");
    let at = diagnostic.primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (78, 79));
}

#[test]
fn private_string_assignment_rejects_a_moved_target() {
    let source = STRING_ASSIGN_MOVE_SOURCE.replacen("\"new\"", "x", 1);
    let sources = sources_for(&source);
    let mut raw = shift_snapshot_signed(response_snapshot(STRING_ASSIGN_MOVE_RESPONSE), 74, -4);
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved assignment target");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3014")
        .expect("moved target diagnostic");
    let at = diagnostic.primary_span().expect("target span");
    assert_eq!((at.start(), at.end()), (72, 73));
}

#[test]
fn private_string_assignment_rejects_a_moved_source() {
    let source = STRING_ASSIGN_MOVE_SOURCE
        .replacen("const y", "let   y", 1)
        .replacen("\"new\"", "x    ", 1)
        .replacen("x = y", "y = x", 1);
    let sources = sources_for(&source);
    let mut raw = response_snapshot(STRING_ASSIGN_MOVE_RESPONSE);
    let RawStatementKind::LocalDeclaration { mutable, keyword_span, .. } =
        &mut raw.files[0].functions[0].body.statements[1].kind
    else {
        panic!("second local")
    };
    *mutable = true;
    keyword_span.end = 54;
    raw.files[0].functions[0].body.expressions[1] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 69, end: 70 },
            },
        },
    };
    for (id, text) in [(2, "y"), (3, "x")] {
        let zryna_syntax::v4::RawExpressionKind::Reference { name } =
            &mut raw.files[0].functions[0].body.expressions[id].kind
        else {
            panic!("assignment reference")
        };
        name.text = text.to_owned();
    }
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved assignment source");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved source diagnostic");
    let at = diagnostic.primary_span().expect("source span");
    assert_eq!((at.start(), at.end()), (80, 81));
}
