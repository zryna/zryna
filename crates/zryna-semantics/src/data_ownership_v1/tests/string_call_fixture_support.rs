use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn private_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    fn append(text: &mut String, spelling: &str) -> zryna_source::UntrustedSpan {
        let start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(spelling);
        zryna_source::UntrustedSpan {
            file: 0,
            start,
            end: u32::try_from(text.len()).expect("fixture offset"),
        }
    }
    fn string_type(text: &mut String, types: &mut Vec<RawTypeSyntax>) -> u32 {
        let keyword_span = append(text, "String");
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: keyword_span,
            kind: RawTypeSyntaxKind::String { keyword_span },
        });
        id
    }

    let mut text = String::new();
    let mut types = Vec::new();
    let mut functions = Vec::new();

    let caller_start = u32::try_from(text.len()).expect("offset");
    let caller_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let caller_name_span = append(&mut text, "caller");
    append(&mut text, "()");
    append(&mut text, ": ");
    let caller_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let caller_body_start = u32::try_from(text.len()).expect("offset");
    let caller_open = append(&mut text, "{");
    append(&mut text, " ");

    let survivor_start = u32::try_from(text.len()).expect("offset");
    let survivor_keyword = append(&mut text, "const");
    append(&mut text, " ");
    let survivor_name = append(&mut text, "survivor");
    append(&mut text, ": ");
    let survivor_type = string_type(&mut text, &mut types);
    append(&mut text, " = ");
    let survivor_literal = append(&mut text, "\"keep\"");
    let survivor_semi = append(&mut text, ";");
    append(&mut text, " ");

    let value_start = u32::try_from(text.len()).expect("offset");
    let value_keyword = append(&mut text, "const");
    append(&mut text, " ");
    let value_name = append(&mut text, "value");
    append(&mut text, ": ");
    let value_type = string_type(&mut text, &mut types);
    append(&mut text, " = ");
    let identity_name = append(&mut text, "identity");
    let identity_open = append(&mut text, "(");
    let producer_name = append(&mut text, "producer");
    let producer_open = append(&mut text, "(");
    let producer_close = append(&mut text, ")");
    let identity_close = append(&mut text, ")");
    let value_semi = append(&mut text, ";");
    append(&mut text, " ");

    let return_start = u32::try_from(text.len()).expect("offset");
    let return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let clone_keyword = append(&mut text, "clone");
    let clone_open = append(&mut text, "(");
    let return_name = append(&mut text, "value");
    let clone_close = append(&mut text, ")");
    let return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let caller_close = append(&mut text, "}");
    let caller_end = caller_close.end;
    let caller_span = zryna_source::UntrustedSpan { file: 0, start: caller_start, end: caller_end };
    let caller_body_span =
        zryna_source::UntrustedSpan { file: 0, start: caller_body_start, end: caller_end };
    functions.push(RawFunctionSyntax {
        span: caller_span,
        export_span: None,
        function_span: caller_keyword,
        name: RawIdentifierSyntax { text: "caller".to_owned(), span: caller_name_span },
        parameters: Vec::new(),
        result_type: caller_result,
        body: RawFunctionBodySyntax {
            span: caller_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: caller_body_span,
                open_brace_span: caller_open,
                statements: vec![0, 1, 2],
                close_brace_span: caller_close,
            }],
            statements: vec![
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: survivor_start,
                        end: survivor_semi.end,
                    },
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: survivor_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax {
                            text: "survivor".to_owned(),
                            span: survivor_name,
                        },
                        type_syntax: survivor_type,
                        equals_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: survivor_literal.start - 2,
                            end: survivor_literal.start - 1,
                        },
                        initializer: 0,
                        semicolon_span: survivor_semi,
                    },
                },
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: value_start,
                        end: value_semi.end,
                    },
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: value_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax { text: "value".to_owned(), span: value_name },
                        type_syntax: value_type,
                        equals_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: identity_name.start - 2,
                            end: identity_name.start - 1,
                        },
                        initializer: 2,
                        semicolon_span: value_semi,
                    },
                },
                RawStatementSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: return_start,
                        end: return_semi.end,
                    },
                    kind: RawStatementKind::Return {
                        keyword_span: return_keyword,
                        value: 4,
                        semicolon_span: return_semi,
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: survivor_literal,
                    kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                        spelling: "\"keep\"".to_owned(),
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: producer_name.start,
                        end: producer_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "producer".to_owned(),
                            span: producer_name,
                        },
                        open_paren_span: producer_open,
                        arguments: Vec::new(),
                        close_paren_span: producer_close,
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: identity_name.start,
                        end: identity_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "identity".to_owned(),
                            span: identity_name,
                        },
                        open_paren_span: identity_open,
                        arguments: vec![1],
                        close_paren_span: identity_close,
                    },
                },
                RawExpressionSyntax {
                    span: return_name,
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax { text: "value".to_owned(), span: return_name },
                    },
                },
                RawExpressionSyntax {
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: clone_keyword.start,
                        end: clone_close.end,
                    },
                    kind: zryna_syntax::v4::RawExpressionKind::Clone {
                        keyword_span: clone_keyword,
                        open_paren_span: clone_open,
                        value: 3,
                        close_paren_span: clone_close,
                    },
                },
            ],
        },
    });

    append(&mut text, " ");
    let identity_start = u32::try_from(text.len()).expect("offset");
    let identity_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let identity_decl_name = append(&mut text, "identity");
    let identity_parameter_open = append(&mut text, "(");
    let parameter_start = u32::try_from(text.len()).expect("offset");
    let parameter_name = append(&mut text, "value");
    append(&mut text, ": ");
    let parameter_type = string_type(&mut text, &mut types);
    let identity_parameter_close = append(&mut text, ")");
    append(&mut text, ": ");
    let identity_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let identity_body_start = u32::try_from(text.len()).expect("offset");
    let identity_body_open = append(&mut text, "{");
    append(&mut text, " ");
    let identity_return_start = u32::try_from(text.len()).expect("offset");
    let identity_return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let identity_reference = append(&mut text, "value");
    let identity_return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let identity_body_close = append(&mut text, "}");
    let identity_end = identity_body_close.end;
    let identity_body_span =
        zryna_source::UntrustedSpan { file: 0, start: identity_body_start, end: identity_end };
    functions.push(RawFunctionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: identity_start, end: identity_end },
        export_span: None,
        function_span: identity_keyword,
        name: RawIdentifierSyntax { text: "identity".to_owned(), span: identity_decl_name },
        parameters: vec![RawParameterSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: parameter_start,
                end: types[usize::try_from(parameter_type).expect("type index")].span.end,
            },
            name: RawIdentifierSyntax { text: "value".to_owned(), span: parameter_name },
            type_syntax: parameter_type,
        }],
        result_type: identity_result,
        body: RawFunctionBodySyntax {
            span: identity_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: identity_body_span,
                open_brace_span: identity_body_open,
                statements: vec![0],
                close_brace_span: identity_body_close,
            }],
            statements: vec![RawStatementSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: identity_return_start,
                    end: identity_return_semi.end,
                },
                kind: RawStatementKind::Return {
                    keyword_span: identity_return_keyword,
                    value: 0,
                    semicolon_span: identity_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: identity_reference,
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "value".to_owned(),
                        span: identity_reference,
                    },
                },
            }],
        },
    });
    let _ = (identity_parameter_open, identity_parameter_close);

    append(&mut text, " ");
    let producer_start = u32::try_from(text.len()).expect("offset");
    let producer_keyword = append(&mut text, "function");
    append(&mut text, " ");
    let producer_decl_name = append(&mut text, "producer");
    append(&mut text, "()");
    append(&mut text, ": ");
    let producer_result = string_type(&mut text, &mut types);
    append(&mut text, " ");
    let producer_body_start = u32::try_from(text.len()).expect("offset");
    let producer_body_open = append(&mut text, "{");
    append(&mut text, " ");
    let producer_return_start = u32::try_from(text.len()).expect("offset");
    let producer_return_keyword = append(&mut text, "return");
    append(&mut text, " ");
    let made_literal = append(&mut text, "\"made\"");
    let producer_return_semi = append(&mut text, ";");
    append(&mut text, " ");
    let producer_body_close = append(&mut text, "}");
    let producer_end = producer_body_close.end;
    let producer_body_span =
        zryna_source::UntrustedSpan { file: 0, start: producer_body_start, end: producer_end };
    functions.push(RawFunctionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: producer_start, end: producer_end },
        export_span: None,
        function_span: producer_keyword,
        name: RawIdentifierSyntax { text: "producer".to_owned(), span: producer_decl_name },
        parameters: Vec::new(),
        result_type: producer_result,
        body: RawFunctionBodySyntax {
            span: producer_body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: producer_body_span,
                open_brace_span: producer_body_open,
                statements: vec![0],
                close_brace_span: producer_body_close,
            }],
            statements: vec![RawStatementSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: producer_return_start,
                    end: producer_return_semi.end,
                },
                kind: RawStatementKind::Return {
                    keyword_span: producer_return_keyword,
                    value: 0,
                    semicolon_span: producer_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: made_literal,
                kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                    spelling: "\"made\"".to_owned(),
                },
            }],
        },
    });

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
                functions,
            }],
            diagnostics: Vec::new(),
        },
    )
}

pub(super) fn private_nested_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = private_string_call_fixture();
    let old = "producer()";
    let replacement = "concat(survivor, \"x\")";
    let start = u32::try_from(source.find(old).expect("producer call")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let body = &mut raw.files[0].functions[0].body;
    let survivor_literal = body.expressions[0].clone();
    let return_reference = body.expressions[3].clone();
    let mut return_clone = body.expressions[4].clone();
    let survivor = zryna_source::UntrustedSpan { file: 0, start: start + 7, end: start + 15 };
    let survivor_reference = RawExpressionSyntax {
        span: survivor,
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "survivor".to_owned(), span: survivor },
        },
    };
    let literal = zryna_source::UntrustedSpan { file: 0, start: start + 17, end: start + 20 };
    let literal_expression = RawExpressionSyntax {
        span: literal,
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"x\"".to_owned() },
    };
    let concat_expression = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 21 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "concat".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 6 },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 6,
                end: start + 7,
            },
            arguments: vec![1, 2],
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 20,
                end: start + 21,
            },
        },
    };
    let mut identity_expression = body.expressions[2].clone();
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity_expression.kind
    else {
        panic!("identity call")
    };
    *arguments = vec![3];
    let zryna_syntax::v4::RawExpressionKind::Clone { value, .. } = &mut return_clone.kind else {
        panic!("return clone")
    };
    *value = 5;
    body.expressions = vec![
        survivor_literal,
        survivor_reference,
        literal_expression,
        concat_expression,
        identity_expression,
        return_reference,
        return_clone,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("value declaration")
    };
    *initializer = 4;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
    (source, raw)
}
