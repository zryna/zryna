use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn nested_owned_partial_local_transfer_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL_PREFIX: &str = "const o: Outer = ";
    const SUFFIX: &str = "const text: String = o.inner.text; const q: Outer = o; return Outer({ tail: \"c\", inner: Inner({ text: \"d\" }) }); ";
    let mut source = NESTED_OWNED_SOURCE.to_owned();
    let return_start = source.find("return Outer").expect("nested return");
    let initializer_start = return_start + "return ".len();
    source.replace_range(return_start..initializer_start, LOCAL_PREFIX);
    let prefix_growth =
        u32::try_from(LOCAL_PREFIX.len() - "return ".len()).expect("nested local prefix growth");
    let mut raw = shift_snapshot(
        response_snapshot(NESTED_OWNED_RESPONSE),
        u32::try_from(initializer_start).expect("nested initializer start"),
        prefix_growth,
    );
    let insertion = source.rfind('}').expect("nested function close");
    source.insert_str(insertion, SUFFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested suffix insertion"),
        u32::try_from(SUFFIX.len()).expect("nested suffix length"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested span start"),
        end: u32::try_from(end).expect("nested span end"),
    };
    let local_outer_start = return_start + "const o: ".len();
    let local_outer_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested local Outer type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(local_outer_start, local_outer_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(local_outer_start, local_outer_start + "Outer".len()),
            },
        },
    });
    let text_statement = insertion;
    let string_start = text_statement + "const text: ".len();
    let string_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested local String type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(string_start, string_start + "String".len()),
        kind: RawTypeSyntaxKind::String {
            keyword_span: s(string_start, string_start + "String".len()),
        },
    });
    let q_statement = source[text_statement..]
        .find("const q:")
        .map(|offset| text_statement + offset)
        .expect("nested q statement");
    let q_type_start = q_statement + "const q: ".len();
    let q_type = u32::try_from(raw.files[0].type_syntax.len()).expect("nested q type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(q_type_start, q_type_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(q_type_start, q_type_start + "Outer".len()),
            },
        },
    });

    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::Return { value: initializer, semicolon_span, .. } =
        body.statements[0].kind
    else {
        panic!("nested original return")
    };
    body.statements[0] = RawStatementSyntax {
        span: s(return_start, usize::try_from(semicolon_span.end).expect("nested local end")),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(return_start, return_start + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(return_start + "const ".len(), return_start + "const o".len()),
            },
            type_syntax: local_outer_type,
            equals_span: s(
                return_start + "const o: Outer ".len(),
                return_start + "const o: Outer =".len(),
            ),
            initializer,
            semicolon_span,
        },
    };

    let projection_start = text_statement + "const text: String = ".len();
    let outer_base = u32::try_from(body.expressions.len()).expect("nested outer base id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(projection_start, projection_start + 1),
            },
        },
    });
    let inner = u32::try_from(body.expressions.len()).expect("nested inner projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + "o.inner".len()),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: outer_base,
            dot_span: s(projection_start + 1, projection_start + 2),
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: s(projection_start + 2, projection_start + "o.inner".len()),
            },
        },
    });
    let nested_text = u32::try_from(body.expressions.len()).expect("nested text projection id");
    body.expressions.push(RawExpressionSyntax {
        span: s(projection_start, projection_start + "o.inner.text".len()),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: inner,
            dot_span: s(projection_start + "o.inner".len(), projection_start + "o.inner.".len()),
            field: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(
                    projection_start + "o.inner.".len(),
                    projection_start + "o.inner.text".len(),
                ),
            },
        },
    });
    let text_semicolon = projection_start + "o.inner.text".len();
    body.statements.push(RawStatementSyntax {
        span: s(text_statement, text_semicolon + 1),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(text_statement, text_statement + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "text".to_owned(),
                span: s(text_statement + "const ".len(), text_statement + "const text".len()),
            },
            type_syntax: string_type,
            equals_span: s(projection_start - 2, projection_start - 1),
            initializer: nested_text,
            semicolon_span: s(text_semicolon, text_semicolon + 1),
        },
    });
    let q_source_start = q_statement + "const q: Outer = ".len();
    let q_source = u32::try_from(body.expressions.len()).expect("nested transfer source id");
    body.expressions.push(RawExpressionSyntax {
        span: s(q_source_start, q_source_start + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: s(q_source_start, q_source_start + 1),
            },
        },
    });
    body.statements.push(RawStatementSyntax {
        span: s(q_statement, q_source_start + 2),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(q_statement, q_statement + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(q_statement + "const ".len(), q_statement + "const q".len()),
            },
            type_syntax: q_type,
            equals_span: s(q_source_start - 2, q_source_start - 1),
            initializer: q_source,
            semicolon_span: s(q_source_start + 1, q_source_start + 2),
        },
    });

    let return_statement = source[q_source_start..]
        .find("return Outer")
        .map(|offset| q_source_start + offset)
        .expect("nested rebuilt return");
    let result_start = return_statement + "return ".len();
    let tail_start = source[result_start..]
        .find("\"c\"")
        .map(|offset| result_start + offset)
        .expect("nested rebuilt tail");
    let text_literal_start = source[tail_start + 3..]
        .find("\"d\"")
        .map(|offset| tail_start + 3 + offset)
        .expect("nested rebuilt text");
    let inner_start = source[result_start..]
        .find("Inner({")
        .map(|offset| result_start + offset)
        .expect("nested rebuilt Inner");
    let result_end = source[result_start..]
        .find(" });")
        .map(|offset| result_start + offset + " })".len())
        .expect("nested rebuilt result end");
    let tail = u32::try_from(body.expressions.len()).expect("nested rebuilt tail id");
    body.expressions.push(RawExpressionSyntax {
        span: s(tail_start, tail_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"c\"".to_owned() },
    });
    let rebuilt_text = u32::try_from(body.expressions.len()).expect("nested rebuilt text id");
    body.expressions.push(RawExpressionSyntax {
        span: s(text_literal_start, text_literal_start + 3),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"d\"".to_owned() },
    });
    let rebuilt_inner = u32::try_from(body.expressions.len()).expect("nested rebuilt Inner id");
    body.expressions.push(RawExpressionSyntax {
        span: s(inner_start, inner_start + "Inner({ text: \"d\" })".len()),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "Inner".to_owned(),
                span: s(inner_start, inner_start + "Inner".len()),
            },
            open_paren_span: s(inner_start + 5, inner_start + 6),
            open_brace_span: s(inner_start + 6, inner_start + 7),
            fields: vec![zryna_syntax::v4::RawFieldInitializer {
                span: s(inner_start + 8, text_literal_start + 3),
                kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                    name: RawIdentifierSyntax {
                        text: "text".to_owned(),
                        span: s(inner_start + 8, inner_start + 12),
                    },
                    colon_span: s(inner_start + 12, inner_start + 13),
                    value: rebuilt_text,
                },
            }],
            close_brace_span: s(inner_start + 18, inner_start + 19),
            close_paren_span: s(inner_start + 19, inner_start + 20),
        },
    });
    let rebuilt_outer = u32::try_from(body.expressions.len()).expect("nested rebuilt Outer id");
    body.expressions.push(RawExpressionSyntax {
        span: s(result_start, result_end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(result_start, result_start + "Outer".len()),
            },
            open_paren_span: s(result_start + 5, result_start + 6),
            open_brace_span: s(result_start + 6, result_start + 7),
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(result_start + 8, tail_start + 3),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "tail".to_owned(),
                            span: s(result_start + 8, result_start + 12),
                        },
                        colon_span: s(result_start + 12, result_start + 13),
                        value: tail,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: s(result_start + 19, inner_start + 20),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "inner".to_owned(),
                            span: s(result_start + 19, result_start + 24),
                        },
                        colon_span: s(result_start + 24, result_start + 25),
                        value: rebuilt_inner,
                    },
                },
            ],
            close_brace_span: s(result_end - 2, result_end - 1),
            close_paren_span: s(result_end - 1, result_end),
        },
    });
    let return_semicolon = result_end;
    body.statements.push(RawStatementSyntax {
        span: s(return_statement, return_semicolon + 1),
        kind: RawStatementKind::Return {
            keyword_span: s(return_statement, return_statement + "return".len()),
            value: rebuilt_outer,
            semicolon_span: s(return_semicolon, return_semicolon + 1),
        },
    });
    body.blocks[0].statements = vec![0, 1, 2, 3];
    (source, raw)
}

pub(super) fn nested_owned_partial_return_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const LOCAL_PREFIX: &str = "const unused: Outer = ";
    const RETURN: &str = "return q; ";
    let (mut source, mut raw) = nested_owned_partial_local_transfer_snapshot();
    let return_start = source.rfind("return Outer").expect("nested partial return source");
    let initializer_start = return_start + "return ".len();
    source.replace_range(return_start..initializer_start, LOCAL_PREFIX);
    raw = shift_snapshot(
        raw,
        u32::try_from(initializer_start).expect("nested partial initializer start"),
        u32::try_from(LOCAL_PREFIX.len() - "return ".len()).expect("nested partial local growth"),
    );
    let s = |start: usize, end: usize| zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("nested partial span start"),
        end: u32::try_from(end).expect("nested partial span end"),
    };
    let type_start = return_start + "const unused: ".len();
    let outer_type =
        u32::try_from(raw.files[0].type_syntax.len()).expect("nested partial unused Outer type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: s(type_start, type_start + "Outer".len()),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "Outer".to_owned(),
                span: s(type_start, type_start + "Outer".len()),
            },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    let last = body.statements.len() - 1;
    let RawStatementKind::Return { value: initializer, semicolon_span, .. } =
        body.statements[last].kind
    else {
        panic!("nested partial original return")
    };
    body.statements[last] = RawStatementSyntax {
        span: s(
            return_start,
            usize::try_from(semicolon_span.end).expect("nested partial local end"),
        ),
        kind: RawStatementKind::LocalDeclaration {
            keyword_span: s(return_start, return_start + "const".len()),
            mutable: false,
            name: RawIdentifierSyntax {
                text: "unused".to_owned(),
                span: s(return_start + "const ".len(), return_start + "const unused".len()),
            },
            type_syntax: outer_type,
            equals_span: s(
                initializer_start + LOCAL_PREFIX.len() - "return ".len() - 2,
                initializer_start + LOCAL_PREFIX.len() - "return ".len() - 1,
            ),
            initializer,
            semicolon_span,
        },
    };

    let insertion = source.rfind('}').expect("nested partial function close");
    source.insert_str(insertion, RETURN);
    raw = shift_snapshot(
        raw,
        u32::try_from(insertion).expect("nested partial return insertion"),
        u32::try_from(RETURN.len()).expect("nested partial return length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let returned = u32::try_from(body.expressions.len()).expect("nested partial q value id");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + "return ".len(), insertion + "return q".len()),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "q".to_owned(),
                span: s(insertion + "return ".len(), insertion + "return q".len()),
            },
        },
    });
    let return_id =
        u32::try_from(body.statements.len()).expect("nested partial return statement id");
    body.statements.push(RawStatementSyntax {
        span: s(insertion, insertion + "return q;".len()),
        kind: RawStatementKind::Return {
            keyword_span: s(insertion, insertion + "return".len()),
            value: returned,
            semicolon_span: s(insertion + "return q".len(), insertion + "return q;".len()),
        },
    });
    body.blocks[0].statements.push(return_id);
    (source, raw)
}
