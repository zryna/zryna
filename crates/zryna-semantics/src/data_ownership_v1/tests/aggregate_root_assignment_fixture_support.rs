use super::*;

#[derive(Clone, Copy)]
pub(super) enum OwnedPairAssignmentRhs {
    Fresh,
    CloneTarget,
    SelfMove,
}
#[allow(clippy::too_many_lines)]
pub(super) fn owned_pair_assignment_snapshot(
    rhs: OwnedPairAssignmentRhs,
    mutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let assignment = match rhs {
        OwnedPairAssignmentRhs::Fresh => "p = OwnedPair({ flag: false, first: \"b\" }); ",
        OwnedPairAssignmentRhs::CloneTarget => "p = clone(p); ",
        OwnedPairAssignmentRhs::SelfMove => "p = p; ",
    };
    let mut source = OWNED_PAIR_SOURCE.to_owned();
    if mutable {
        source.replace_range(100..105, "let  ");
    }
    let insertion = source.find("return p;").expect("return insertion");
    source.insert_str(insertion, assignment);
    let insertion = u32::try_from(insertion).expect("fixture insertion");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_PAIR_RESPONSE),
        insertion,
        u32::try_from(assignment.len()).expect("assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable: is_mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    if mutable {
        keyword_span.end = keyword_span.start + 3;
        *is_mutable = true;
    }
    let target = u32::try_from(body.expressions.len()).expect("target expression");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
            },
        },
    });
    let value = match rhs {
        OwnedPairAssignmentRhs::Fresh => {
            let bool_value = u32::try_from(body.expressions.len()).expect("bool value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 22,
                    end: insertion + 27,
                },
                kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
            });
            let string_value = u32::try_from(body.expressions.len()).expect("String value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 36,
                    end: insertion + 39,
                },
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"b\"".to_owned(),
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("Struct value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 42,
                },
                kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
                    type_name: RawIdentifierSyntax {
                        text: "OwnedPair".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 4,
                            end: insertion + 13,
                        },
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 13,
                        end: insertion + 14,
                    },
                    open_brace_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 14,
                        end: insertion + 15,
                    },
                    fields: vec![
                        zryna_syntax::v4::RawFieldInitializer {
                            span: zryna_source::UntrustedSpan {
                                file: 0,
                                start: insertion + 16,
                                end: insertion + 27,
                            },
                            kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                                name: RawIdentifierSyntax {
                                    text: "flag".to_owned(),
                                    span: zryna_source::UntrustedSpan {
                                        file: 0,
                                        start: insertion + 16,
                                        end: insertion + 20,
                                    },
                                },
                                colon_span: zryna_source::UntrustedSpan {
                                    file: 0,
                                    start: insertion + 20,
                                    end: insertion + 21,
                                },
                                value: bool_value,
                            },
                        },
                        zryna_syntax::v4::RawFieldInitializer {
                            span: zryna_source::UntrustedSpan {
                                file: 0,
                                start: insertion + 29,
                                end: insertion + 39,
                            },
                            kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                                name: RawIdentifierSyntax {
                                    text: "first".to_owned(),
                                    span: zryna_source::UntrustedSpan {
                                        file: 0,
                                        start: insertion + 29,
                                        end: insertion + 34,
                                    },
                                },
                                colon_span: zryna_source::UntrustedSpan {
                                    file: 0,
                                    start: insertion + 34,
                                    end: insertion + 35,
                                },
                                value: string_value,
                            },
                        },
                    ],
                    close_brace_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 40,
                        end: insertion + 41,
                    },
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 41,
                        end: insertion + 42,
                    },
                },
            });
            value
        }
        OwnedPairAssignmentRhs::CloneTarget => {
            let source_value = u32::try_from(body.expressions.len()).expect("clone source");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 10,
                    end: insertion + 11,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 10,
                            end: insertion + 11,
                        },
                    },
                },
            });
            let value = u32::try_from(body.expressions.len()).expect("clone value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 12,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Clone {
                    keyword_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 4,
                        end: insertion + 9,
                    },
                    open_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 9,
                        end: insertion + 10,
                    },
                    value: source_value,
                    close_paren_span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: insertion + 11,
                        end: insertion + 12,
                    },
                },
            });
            value
        }
        OwnedPairAssignmentRhs::SelfMove => {
            let value = u32::try_from(body.expressions.len()).expect("self-move value");
            body.expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 4,
                    end: insertion + 5,
                },
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "p".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: insertion + 4,
                            end: insertion + 5,
                        },
                    },
                },
            });
            value
        }
    };
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion,
                end: insertion + u32::try_from(assignment.trim_end().len()).expect("statement"),
            },
            kind: RawStatementKind::Assignment {
                target,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 2,
                    end: insertion + 3,
                },
                value,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion
                        + u32::try_from(assignment.trim_end().len() - 1).expect("semicolon"),
                    end: insertion + u32::try_from(assignment.trim_end().len()).expect("semicolon"),
                },
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}
#[derive(Clone, Copy)]
pub(super) enum OwnedPairProjectionAssignmentRhs {
    CopyField,
    MoveField,
}
pub(super) fn owned_pair_projection_assignment_snapshot(
    rhs: OwnedPairProjectionAssignmentRhs,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::Fresh, true);
    let (old, replacement) = match rhs {
        OwnedPairProjectionAssignmentRhs::CopyField => ("false", "p.flag"),
        OwnedPairProjectionAssignmentRhs::MoveField => ("\"b\"", "p.first"),
    };
    let start = source.find(old).expect("projected assignment operand");
    source.replace_range(start..start + old.len(), replacement);
    let start = u32::try_from(start).expect("projected operand offset");
    let delta = i32::try_from(replacement.len()).expect("replacement length")
        - i32::try_from(old.len()).expect("old length");
    let mut raw = shift_snapshot_signed(
        raw,
        start + u32::try_from(old.len()).expect("old operand end"),
        delta,
    );
    let body = &mut raw.files[0].functions[0].body;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    match rhs {
        OwnedPairProjectionAssignmentRhs::CopyField => {
            body.expressions[5] = RawExpressionSyntax {
                span: s(start, start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start, start + 1) },
                },
            };
            body.expressions.insert(
                6,
                RawExpressionSyntax {
                    span: s(start, start + 6),
                    kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                        base: 5,
                        dot_span: s(start + 1, start + 2),
                        field: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(start + 2, start + 6),
                        },
                    },
                },
            );
            let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
                &mut body.expressions[8].kind
            else {
                panic!("projected assignment Struct")
            };
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[0].kind
            else {
                panic!("flag initializer")
            };
            *value = 6;
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[1].kind
            else {
                panic!("first initializer")
            };
            *value = 7;
        }
        OwnedPairProjectionAssignmentRhs::MoveField => {
            body.expressions[6] = RawExpressionSyntax {
                span: s(start, start + 1),
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "p".to_owned(), span: s(start, start + 1) },
                },
            };
            body.expressions.insert(
                7,
                RawExpressionSyntax {
                    span: s(start, start + 7),
                    kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
                        base: 6,
                        dot_span: s(start + 1, start + 2),
                        field: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(start + 2, start + 7),
                        },
                    },
                },
            );
            let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
                &mut body.expressions[8].kind
            else {
                panic!("projected assignment Struct")
            };
            let zryna_syntax::v4::RawFieldInitializerKind::Explicit { value, .. } =
                &mut fields[1].kind
            else {
                panic!("first initializer")
            };
            *value = 7;
        }
    }
    let RawStatementKind::Assignment { value, .. } = &mut body.statements[1].kind else {
        panic!("projected aggregate assignment")
    };
    *value = 8;
    (source, raw)
}
pub(super) fn owned_enum_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    let source = OWNED_ENUM_STRING_SOURCE
        .replacen("const x", "let   x", 1)
        .replacen("const y: Maybe = x;", "x = Maybe.none();  ", 1)
        .replacen("return y", "return x", 1);
    let mut raw = response_snapshot(OWNED_ENUM_STRING_RESPONSE);
    assert_eq!(raw.files[0].type_syntax.len(), 5);
    raw.files[0].type_syntax.pop();
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[1].kind
    else {
        panic!("enum target local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let assignment = u32::try_from(source.find("x = Maybe.none()").expect("enum assignment"))
        .expect("enum assignment span");
    body.expressions[3] = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment, end: assignment + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment,
                    end: assignment + 1,
                },
            },
        },
    };
    let replacement = u32::try_from(body.expressions.len()).expect("enum replacement");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment + 4, end: assignment + 16 },
        kind: zryna_syntax::v4::RawExpressionKind::EnumConstruction {
            type_name: RawIdentifierSyntax {
                text: "Maybe".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment + 4,
                    end: assignment + 9,
                },
            },
            dot_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 9,
                end: assignment + 10,
            },
            variant: RawIdentifierSyntax {
                text: "none".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: assignment + 10,
                    end: assignment + 14,
                },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 14,
                end: assignment + 15,
            },
            payload: None,
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 15,
                end: assignment + 16,
            },
        },
    });
    body.statements[2] = RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: assignment, end: assignment + 17 },
        kind: RawStatementKind::Assignment {
            target: 3,
            equals_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 2,
                end: assignment + 3,
            },
            value: replacement,
            semicolon_span: zryna_source::UntrustedSpan {
                file: 0,
                start: assignment + 16,
                end: assignment + 17,
            },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut body.expressions[4].kind
    else {
        panic!("enum return")
    };
    name.text = "x".to_owned();
    (source, raw)
}
