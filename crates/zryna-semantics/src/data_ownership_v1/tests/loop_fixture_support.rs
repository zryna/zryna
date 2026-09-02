use super::*;

#[allow(clippy::too_many_lines)]
pub(super) fn private_string_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    private_string_loop_fixture_with_options(false, false, false)
}

#[allow(clippy::too_many_lines)]
pub(super) fn private_string_loop_fixture_with_incoming_move(
    move_incoming: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    private_string_loop_fixture_with_options(move_incoming, false, false)
}

#[allow(clippy::too_many_lines)]
pub(super) fn private_string_loop_fixture_with_options(
    move_incoming: bool,
    non_bool_condition: bool,
    false_condition: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let first_initializer = if move_incoming { "outer" } else { "\"a\"" };
    let condition = if non_bool_condition {
        "outer"
    } else if false_condition {
        "false"
    } else {
        "flag"
    };
    let text = format!(
        "function keep(flag: bool): String {{ const outer: String = \"keep\"; while ({condition}) {{ const first: String = {first_initializer}; const second: String = \"b\"; }} return outer; }}"
    );
    let token = |needle, ordinal| nth_untrusted_span(&text, needle, ordinal);
    let range = |start, start_ordinal, end, end_ordinal| {
        untrusted_range(&text, (start, start_ordinal), (end, end_ordinal))
    };
    let bool_span = token("bool", 0);
    let string_spans = (0..4).map(|ordinal| token("String", ordinal)).collect::<Vec<_>>();
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
    let root_span = range("{", 0, "}", 1);
    let body_span = range("{", 1, "}", 0);
    let statements = vec![
        RawStatementSyntax {
            span: range("const outer", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
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
            span: range("const first", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: 3,
                equals_span: token("=", 1),
                initializer: 2,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("const second", 0, ";", 2),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: 4,
                equals_span: token("=", 2),
                initializer: 3,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 3),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 4,
                semicolon_span: token(";", 3),
            },
        },
    ];
    let expressions = vec![
        RawExpressionSyntax {
            span: token("\"keep\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"keep\"".to_owned() },
        },
        if non_bool_condition {
            RawExpressionSyntax {
                span: token("outer", 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
                },
            }
        } else if false_condition {
            RawExpressionSyntax {
                span: token("false", 0),
                kind: RawExpressionKind::BoolLiteral { value: false },
            }
        } else {
            RawExpressionSyntax {
                span: token("flag", 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax { text: "flag".to_owned(), span: token("flag", 1) },
                },
            }
        },
        if move_incoming {
            RawExpressionSyntax {
                span: token("outer", usize::from(non_bool_condition) + 1),
                kind: RawExpressionKind::Reference {
                    name: RawIdentifierSyntax {
                        text: "outer".to_owned(),
                        span: token("outer", usize::from(non_bool_condition) + 1),
                    },
                },
            }
        } else {
            RawExpressionSyntax {
                span: token("\"a\"", 0),
                kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
            }
        },
        RawExpressionSyntax {
            span: token("\"b\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
        },
        RawExpressionSyntax {
            span: token("outer", usize::from(non_bool_condition) + usize::from(move_incoming) + 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax {
                    text: "outer".to_owned(),
                    span: token(
                        "outer",
                        usize::from(non_bool_condition) + usize::from(move_incoming) + 1,
                    ),
                },
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
                    statements: vec![0, 1, 4],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
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

#[allow(clippy::too_many_lines)]
pub(super) fn private_vec_loop_fixture() -> (String, RawProjectSyntaxSnapshot) {
    use zryna_syntax::v4::RawExpressionKind;
    let text = "function keep(flag: bool): Vec<i32> { const outer: Vec<i32> = Vec<i32>([]); while (flag) { const first: Vec<i32> = Vec<i32>([1]); const second: Vec<i32> = Vec<i32>([2]); } return outer; }".to_owned();
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
    for ordinal in 0..7 {
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
    let one_brackets = token("[1]", 0);
    let one_span = zryna_source::UntrustedSpan {
        file: 0,
        start: one_brackets.start + 1,
        end: one_brackets.end - 1,
    };
    let two_brackets = token("[2]", 0);
    let two_span = zryna_source::UntrustedSpan {
        file: 0,
        start: two_brackets.start + 1,
        end: two_brackets.end - 1,
    };
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
            span: one_span,
            kind: RawExpressionKind::I32Literal { spelling: "1".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 4, ")", 3),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[4],
                open_paren_span: token("(", 3),
                open_bracket_span: token("[", 1),
                elements: vec![2],
                close_bracket_span: token("]", 1),
                close_paren_span: token(")", 3),
            },
        },
        RawExpressionSyntax {
            span: two_span,
            kind: RawExpressionKind::I32Literal { spelling: "2".to_owned() },
        },
        RawExpressionSyntax {
            span: range("Vec<i32>", 6, ")", 4),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: vec_types[6],
                open_paren_span: token("(", 4),
                open_bracket_span: token("[", 2),
                elements: vec![4],
                close_bracket_span: token("]", 2),
                close_paren_span: token(")", 4),
            },
        },
        RawExpressionSyntax {
            span: token("outer", 1),
            kind: RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: "outer".to_owned(), span: token("outer", 1) },
            },
        },
    ];
    let statements = vec![
        RawStatementSyntax {
            span: range("const outer", 0, ";", 0),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 0),
                mutable: false,
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
            span: range("const first", 0, ";", 1),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 1),
                mutable: false,
                name: RawIdentifierSyntax { text: "first".to_owned(), span: token("first", 0) },
                type_syntax: vec_types[3],
                equals_span: token("=", 1),
                initializer: 3,
                semicolon_span: token(";", 1),
            },
        },
        RawStatementSyntax {
            span: range("const second", 0, ";", 2),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: token("const", 2),
                mutable: false,
                name: RawIdentifierSyntax { text: "second".to_owned(), span: token("second", 0) },
                type_syntax: vec_types[5],
                equals_span: token("=", 2),
                initializer: 5,
                semicolon_span: token(";", 2),
            },
        },
        RawStatementSyntax {
            span: range("return", 0, ";", 3),
            kind: RawStatementKind::Return {
                keyword_span: token("return", 0),
                value: 6,
                semicolon_span: token(";", 3),
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
                    statements: vec![0, 1, 4],
                    close_brace_span: token("}", 1),
                },
                RawBlockSyntax {
                    span: body_span,
                    open_brace_span: token("{", 1),
                    statements: vec![2, 3],
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
