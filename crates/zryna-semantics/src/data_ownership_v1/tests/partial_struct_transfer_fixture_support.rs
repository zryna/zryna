use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn owned_pair_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const q: OwnedPair = p; ";
    const PREFIX: &str = "OwnedPair({ flag: ";
    const SUFFIX: &str = ".flag, first: text })";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let insertion = source.find("return p;").expect("partial transfer insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("partial transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCAL.len()).expect("partial transfer local length"),
    );

    let return_start = source.find("return p;").expect("shifted Pair return");
    let expression_start = return_start + "return ".len();
    source.insert_str(expression_start, PREFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(expression_start).expect("return expression start"),
        u32::try_from(PREFIX.len()).expect("return prefix length"),
    );
    let q_start = expression_start + PREFIX.len();
    source.replace_range(q_start..=q_start, "q");
    source.insert_str(q_start + 1, SUFFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(q_start + 1).expect("return suffix start"),
        u32::try_from(SUFFIX.len()).expect("return suffix length"),
    );

    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("span start"),
        end: u32::try_from(end).expect("span end"),
    };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion as usize + 9, insertion as usize + 18),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(insertion as usize + 9, insertion as usize + 18),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_value = u32::try_from(body.expressions.len()).expect("partial source value");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion as usize + 21, insertion as usize + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "p".to_owned(),
                span: s(insertion as usize + 21, insertion as usize + 22),
            },
        },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion as usize, insertion as usize + 23),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion as usize, insertion as usize + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "q".to_owned(),
                    span: s(insertion as usize + 6, insertion as usize + 7),
                },
                type_syntax: pair_type,
                equals_span: s(insertion as usize + 19, insertion as usize + 20),
                initializer: source_value,
                semicolon_span: s(insertion as usize + 22, insertion as usize + 23),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];

    let RawStatementKind::Return { value: old_return, .. } = body.statements[3].kind else {
        panic!("partial transfer return")
    };
    body.expressions[old_return as usize] = RawExpressionSyntax {
        span: s(q_start, q_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "q".to_owned(), span: s(q_start, q_start + 1) },
        },
    };
    let flag = u32::try_from(body.expressions.len()).expect("q.flag expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(q_start, q_start + 6),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: old_return,
            dot_span: s(q_start + 1, q_start + 2),
            field: RawIdentifierSyntax {
                text: "flag".to_owned(),
                span: s(q_start + 2, q_start + 6),
            },
        },
    });
    let text_start = q_start + 15;
    let text = u32::try_from(body.expressions.len()).expect("text expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(text_start, text_start + 4),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(text_start, text_start + 4),
            },
        },
    });
    let expression_end = q_start + 1 + SUFFIX.len();
    let result = u32::try_from(body.expressions.len()).expect("rebuilt Pair expression");
    body.expressions.push(RawExpressionSyntax {
        span: s(expression_start, expression_end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(expression_start, expression_start + 9),
            },
            open_paren_span: s(expression_start + 9, expression_start + 10),
            open_brace_span: s(expression_start + 10, expression_start + 11),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(expression_start + 12, q_start + 6),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "flag".to_owned(),
                            span: s(expression_start + 12, expression_start + 16),
                        },
                        colon_span: s(expression_start + 16, expression_start + 17),
                        value: flag,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(q_start + 8, text_start + 4),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "first".to_owned(),
                            span: s(q_start + 8, q_start + 13),
                        },
                        colon_span: s(q_start + 13, q_start + 14),
                        value: text,
                    },
                },
            ],
            close_brace_span: s(expression_end - 2, expression_end - 1),
            close_paren_span: s(expression_end - 1, expression_end),
        },
    });
    let RawStatementKind::Return { value, .. } = &mut body.statements[3].kind else {
        panic!("partial transfer return")
    };
    *value = result;
    (source, raw)
}

pub(super) fn owned_pair_partial_transfer_then_use_source_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = owned_pair_partial_local_transfer_snapshot();
    let q_start = source.find("flag: q.flag").expect("transferred return owner") + "flag: ".len();
    source.replace_range(q_start..=q_start, "p");
    let q_start = u32::try_from(q_start).expect("return owner offset");
    let expression = raw.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find(|expression| {
            expression.span.start == q_start
                && matches!(
                    &expression.kind,
                    zryna_syntax::v4::RawExpressionKind::Reference { name }
                        if name.text == "q"
                )
        })
        .expect("transferred return owner expression");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expression.kind else {
        unreachable!("filtered return owner expression")
    };
    name.text = "p".to_owned();
    (source, raw)
}

pub(super) fn owned_pair_repeated_partial_local_transfer_snapshot()
-> (String, RawProjectSyntaxSnapshot) {
    const LOCAL: &str = "const r: OwnedPair = q; ";
    let (mut source, mut raw) = owned_pair_partial_local_transfer_snapshot();
    let insertion = source.find("return OwnedPair").expect("repeated transfer insertion");
    source.insert_str(insertion, LOCAL);
    let insertion = u32::try_from(insertion).expect("repeated transfer offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(LOCAL.len()).expect("repeated transfer local length"),
    );
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let pair_type = u32::try_from(raw.files[0].type_syntax.len()).expect("Pair type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(insertion + 9, insertion + 18),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "OwnedPair".to_owned(),
                span: s(insertion + 9, insertion + 18),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let source_value = u32::try_from(body.expressions.len()).expect("repeated source value");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 21, insertion + 22),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + 21, insertion + 22),
            },
        },
    });
    body.statements.insert(
        3,
        RawStatementSyntax {
            span: s(insertion, insertion + 23),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s(insertion, insertion + 5),
                mutable: false,
                name: RawIdentifierSyntax {
                    text: "r".to_owned(),
                    span: s(insertion + 6, insertion + 7),
                },
                type_syntax: pair_type,
                equals_span: s(insertion + 19, insertion + 20),
                initializer: source_value,
                semicolon_span: s(insertion + 22, insertion + 23),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3, 4];
    let q_start = source.find("flag: q.flag").expect("repeated return owner") + "flag: ".len();
    source.replace_range(q_start..=q_start, "r");
    let q_start = u32::try_from(q_start).expect("repeated return owner offset");
    let expression = body
        .expressions
        .iter_mut()
        .find(|expression| {
            expression.span.start == q_start
                && matches!(
                    &expression.kind,
                    zryna_syntax::v4::RawExpressionKind::Reference { name }
                        if name.text == "q"
                )
        })
        .expect("repeated return owner expression");
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expression.kind else {
        unreachable!("filtered repeated owner expression")
    };
    name.text = "r".to_owned();
    (source, raw)
}
