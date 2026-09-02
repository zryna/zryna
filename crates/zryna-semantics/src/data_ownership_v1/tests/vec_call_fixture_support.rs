use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn private_vec_call_fixture(element: &str) -> (String, RawProjectSyntaxSnapshot) {
    fn take(source: &str, cursor: &mut usize, spelling: &str) -> zryna_source::UntrustedSpan {
        let relative = source[*cursor..].find(spelling).expect("fixture token");
        let start = *cursor + relative;
        let end = start + spelling.len();
        *cursor = end;
        zryna_source::UntrustedSpan {
            file: 0,
            start: u32::try_from(start).expect("span"),
            end: u32::try_from(end).expect("span"),
        }
    }
    fn vec_type(
        source: &str,
        cursor: &mut usize,
        types: &mut Vec<RawTypeSyntax>,
        element: &str,
    ) -> u32 {
        let keyword_span = take(source, cursor, "Vec");
        let less_than_span = take(source, cursor, "<");
        let element_span = take(source, cursor, element);
        let element_id = u32::try_from(types.len()).expect("type id");
        let kind = if element == "String" {
            RawTypeSyntaxKind::String { keyword_span: element_span }
        } else {
            RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: element.to_owned(), span: element_span },
            }
        };
        types.push(RawTypeSyntax { span: element_span, kind });
        let greater_than_span = take(source, cursor, ">");
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: keyword_span.start,
                end: greater_than_span.end,
            },
            kind: RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element_id,
                greater_than_span,
            },
        });
        id
    }
    fn joined(start: u32, end: u32) -> zryna_source::UntrustedSpan {
        zryna_source::UntrustedSpan { file: 0, start, end }
    }

    let source = format!(
        "function caller(): Vec<{element}> {{ const survivor: Vec<{element}> = Vec<{element}>([]); const result: Vec<{element}> = identity(producer()); return result; }} function identity(value: Vec<{element}>): Vec<{element}> {{ return value; }} function producer(): Vec<{element}> {{ return Vec<{element}>([]); }}"
    );
    let mut cursor = 0;
    let mut types = Vec::new();
    let mut functions = Vec::new();

    let caller_keyword = take(&source, &mut cursor, "function");
    let caller_name = take(&source, &mut cursor, "caller");
    let caller_result = vec_type(&source, &mut cursor, &mut types, element);
    let caller_open = take(&source, &mut cursor, "{");
    let survivor_keyword = take(&source, &mut cursor, "const");
    let survivor_name = take(&source, &mut cursor, "survivor");
    let survivor_type = vec_type(&source, &mut cursor, &mut types, element);
    let survivor_equals = take(&source, &mut cursor, "=");
    let survivor_construct_type = vec_type(&source, &mut cursor, &mut types, element);
    let survivor_construct_open = take(&source, &mut cursor, "(");
    let survivor_bracket_open = take(&source, &mut cursor, "[");
    let survivor_bracket_close = take(&source, &mut cursor, "]");
    let survivor_construct_close = take(&source, &mut cursor, ")");
    let survivor_semi = take(&source, &mut cursor, ";");
    let result_keyword = take(&source, &mut cursor, "const");
    let result_name = take(&source, &mut cursor, "result");
    let result_type = vec_type(&source, &mut cursor, &mut types, element);
    let result_equals = take(&source, &mut cursor, "=");
    let identity_call_name = take(&source, &mut cursor, "identity");
    let identity_call_open = take(&source, &mut cursor, "(");
    let producer_call_name = take(&source, &mut cursor, "producer");
    let producer_call_open = take(&source, &mut cursor, "(");
    let producer_call_close = take(&source, &mut cursor, ")");
    let identity_call_close = take(&source, &mut cursor, ")");
    let result_semi = take(&source, &mut cursor, ";");
    let caller_return_keyword = take(&source, &mut cursor, "return");
    let caller_return_name = take(&source, &mut cursor, "result");
    let caller_return_semi = take(&source, &mut cursor, ";");
    let caller_close = take(&source, &mut cursor, "}");
    let caller_body = joined(caller_open.start, caller_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(caller_keyword.start, caller_close.end),
        export_span: None,
        function_span: caller_keyword,
        name: RawIdentifierSyntax { text: "caller".to_owned(), span: caller_name },
        parameters: Vec::new(),
        result_type: caller_result,
        body: RawFunctionBodySyntax {
            span: caller_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: caller_body,
                open_brace_span: caller_open,
                statements: vec![0, 1, 2],
                close_brace_span: caller_close,
            }],
            statements: vec![
                RawStatementSyntax {
                    span: joined(survivor_keyword.start, survivor_semi.end),
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: survivor_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax {
                            text: "survivor".to_owned(),
                            span: survivor_name,
                        },
                        type_syntax: survivor_type,
                        equals_span: survivor_equals,
                        initializer: 0,
                        semicolon_span: survivor_semi,
                    },
                },
                RawStatementSyntax {
                    span: joined(result_keyword.start, result_semi.end),
                    kind: RawStatementKind::LocalDeclaration {
                        keyword_span: result_keyword,
                        mutable: false,
                        name: RawIdentifierSyntax { text: "result".to_owned(), span: result_name },
                        type_syntax: result_type,
                        equals_span: result_equals,
                        initializer: 2,
                        semicolon_span: result_semi,
                    },
                },
                RawStatementSyntax {
                    span: joined(caller_return_keyword.start, caller_return_semi.end),
                    kind: RawStatementKind::Return {
                        keyword_span: caller_return_keyword,
                        value: 3,
                        semicolon_span: caller_return_semi,
                    },
                },
            ],
            expressions: vec![
                RawExpressionSyntax {
                    span: joined(
                        survivor_construct_type_span(&types, survivor_construct_type).start,
                        survivor_construct_close.end,
                    ),
                    kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
                        type_syntax: survivor_construct_type,
                        open_paren_span: survivor_construct_open,
                        open_bracket_span: survivor_bracket_open,
                        elements: Vec::new(),
                        close_bracket_span: survivor_bracket_close,
                        close_paren_span: survivor_construct_close,
                    },
                },
                RawExpressionSyntax {
                    span: joined(producer_call_name.start, producer_call_close.end),
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "producer".to_owned(),
                            span: producer_call_name,
                        },
                        open_paren_span: producer_call_open,
                        arguments: Vec::new(),
                        close_paren_span: producer_call_close,
                    },
                },
                RawExpressionSyntax {
                    span: joined(identity_call_name.start, identity_call_close.end),
                    kind: zryna_syntax::v4::RawExpressionKind::Call {
                        callee: RawIdentifierSyntax {
                            text: "identity".to_owned(),
                            span: identity_call_name,
                        },
                        open_paren_span: identity_call_open,
                        arguments: vec![1],
                        close_paren_span: identity_call_close,
                    },
                },
                RawExpressionSyntax {
                    span: caller_return_name,
                    kind: zryna_syntax::v4::RawExpressionKind::Reference {
                        name: RawIdentifierSyntax {
                            text: "result".to_owned(),
                            span: caller_return_name,
                        },
                    },
                },
            ],
        },
    });

    let identity_keyword = take(&source, &mut cursor, "function");
    let identity_name = take(&source, &mut cursor, "identity");
    let parameter_start = take(&source, &mut cursor, "value");
    let parameter_type = vec_type(&source, &mut cursor, &mut types, element);
    let identity_result = vec_type(&source, &mut cursor, &mut types, element);
    let identity_open = take(&source, &mut cursor, "{");
    let identity_return_keyword = take(&source, &mut cursor, "return");
    let identity_return_name = take(&source, &mut cursor, "value");
    let identity_return_semi = take(&source, &mut cursor, ";");
    let identity_close = take(&source, &mut cursor, "}");
    let identity_body = joined(identity_open.start, identity_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(identity_keyword.start, identity_close.end),
        export_span: None,
        function_span: identity_keyword,
        name: RawIdentifierSyntax { text: "identity".to_owned(), span: identity_name },
        parameters: vec![RawParameterSyntax {
            span: joined(
                parameter_start.start,
                types[usize::try_from(parameter_type).expect("type")].span.end,
            ),
            name: RawIdentifierSyntax { text: "value".to_owned(), span: parameter_start },
            type_syntax: parameter_type,
        }],
        result_type: identity_result,
        body: RawFunctionBodySyntax {
            span: identity_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: identity_body,
                open_brace_span: identity_open,
                statements: vec![0],
                close_brace_span: identity_close,
            }],
            statements: vec![RawStatementSyntax {
                span: joined(identity_return_keyword.start, identity_return_semi.end),
                kind: RawStatementKind::Return {
                    keyword_span: identity_return_keyword,
                    value: 0,
                    semicolon_span: identity_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: identity_return_name,
                kind: zryna_syntax::v4::RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "value".to_owned(),
                        span: identity_return_name,
                    },
                },
            }],
        },
    });

    let producer_keyword = take(&source, &mut cursor, "function");
    let producer_name = take(&source, &mut cursor, "producer");
    let producer_result = vec_type(&source, &mut cursor, &mut types, element);
    let producer_open = take(&source, &mut cursor, "{");
    let producer_return_keyword = take(&source, &mut cursor, "return");
    let producer_construct_type = vec_type(&source, &mut cursor, &mut types, element);
    let producer_construct_open = take(&source, &mut cursor, "(");
    let producer_bracket_open = take(&source, &mut cursor, "[");
    let producer_bracket_close = take(&source, &mut cursor, "]");
    let producer_construct_close = take(&source, &mut cursor, ")");
    let producer_return_semi = take(&source, &mut cursor, ";");
    let producer_close = take(&source, &mut cursor, "}");
    let producer_body = joined(producer_open.start, producer_close.end);
    functions.push(RawFunctionSyntax {
        span: joined(producer_keyword.start, producer_close.end),
        export_span: None,
        function_span: producer_keyword,
        name: RawIdentifierSyntax { text: "producer".to_owned(), span: producer_name },
        parameters: Vec::new(),
        result_type: producer_result,
        body: RawFunctionBodySyntax {
            span: producer_body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: producer_body,
                open_brace_span: producer_open,
                statements: vec![0],
                close_brace_span: producer_close,
            }],
            statements: vec![RawStatementSyntax {
                span: joined(producer_return_keyword.start, producer_return_semi.end),
                kind: RawStatementKind::Return {
                    keyword_span: producer_return_keyword,
                    value: 0,
                    semicolon_span: producer_return_semi,
                },
            }],
            expressions: vec![RawExpressionSyntax {
                span: joined(
                    types[usize::try_from(producer_construct_type).expect("type")].span.start,
                    producer_construct_close.end,
                ),
                kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
                    type_syntax: producer_construct_type,
                    open_paren_span: producer_construct_open,
                    open_bracket_span: producer_bracket_open,
                    elements: Vec::new(),
                    close_bracket_span: producer_bracket_close,
                    close_paren_span: producer_construct_close,
                },
            }],
        },
    });

    (
        source,
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

#[allow(clippy::too_many_lines)]
pub(super) fn private_vec_nested_string_call_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, raw) = private_vec_call_fixture("String");
    let old = "producer()";
    let replacement = "Vec<String>([concat(\"a\", \"b\")])";
    let start = u32::try_from(source.find(old).expect("producer call")).expect("offset");
    let old_end = start + u32::try_from(old.len()).expect("length");
    let extra = u32::try_from(replacement.len() - old.len()).expect("growth");
    source.replace_range(
        usize::try_from(start).expect("offset")..usize::try_from(old_end).expect("offset"),
        replacement,
    );
    let mut raw = shift_snapshot(raw, old_end, extra);
    let survivor_construct = raw.files[0].functions[0].body.expressions[0].clone();
    let mut identity = raw.files[0].functions[0].body.expressions[2].clone();
    let return_reference = raw.files[0].functions[0].body.expressions[3].clone();
    let string_span = zryna_source::UntrustedSpan { file: 0, start: start + 4, end: start + 10 };
    let string_type = u32::try_from(raw.files[0].type_syntax.len()).expect("type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: string_span,
        kind: RawTypeSyntaxKind::String { keyword_span: string_span },
    });
    let vec_type = u32::try_from(raw.files[0].type_syntax.len()).expect("type id");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 11 },
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 3 },
            less_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 3,
                end: start + 4,
            },
            argument: string_type,
            greater_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 10,
                end: start + 11,
            },
        },
    });
    let literal = |offset, spelling: &str| RawExpressionSyntax {
        span: zryna_source::UntrustedSpan {
            file: 0,
            start: start + offset,
            end: start + offset + 3,
        },
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: spelling.to_owned() },
    };
    let concat = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: start + 13, end: start + 29 },
        kind: zryna_syntax::v4::RawExpressionKind::Call {
            callee: RawIdentifierSyntax {
                text: "concat".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: start + 13, end: start + 19 },
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 19,
                end: start + 20,
            },
            arguments: vec![1, 2],
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 28,
                end: start + 29,
            },
        },
    };
    let construct = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start, end: start + 31 },
        kind: zryna_syntax::v4::RawExpressionKind::VecConstruction {
            type_syntax: vec_type,
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 11,
                end: start + 12,
            },
            open_bracket_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 12,
                end: start + 13,
            },
            elements: vec![3],
            close_bracket_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 29,
                end: start + 30,
            },
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: start + 30,
                end: start + 31,
            },
        },
    };
    let zryna_syntax::v4::RawExpressionKind::Call { arguments, .. } = &mut identity.kind else {
        panic!("identity call")
    };
    *arguments = vec![4];
    let body = &mut raw.files[0].functions[0].body;
    body.expressions = vec![
        survivor_construct,
        literal(20, "\"a\""),
        literal(25, "\"b\""),
        concat,
        construct,
        identity,
        return_reference,
    ];
    let RawStatementKind::LocalDeclaration { initializer, .. } = &mut body.statements[1].kind
    else {
        panic!("result declaration")
    };
    *initializer = 5;
    let RawStatementKind::Return { value, .. } = &mut body.statements[2].kind else {
        panic!("return")
    };
    *value = 6;
    (source, raw)
}
