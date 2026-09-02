use super::*;

fn derived_value_fixture(result_count: usize) -> RawFunctionSyntax {
    assert!(result_count > 0);
    let span = zryna_source::UntrustedSpan { file: 0, start: 0, end: 1 };
    let child_count = result_count - 1;
    let mut expressions = (0..child_count)
        .map(|_| RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
        })
        .collect::<Vec<_>>();
    expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: 0,
            open_paren_span: span,
            open_bracket_span: span,
            elements: (0..u32::try_from(child_count).expect("fixture expression ids")).collect(),
            close_bracket_span: span,
            close_paren_span: span,
        },
    });
    RawFunctionSyntax {
        span,
        export_span: None,
        function_span: span,
        name: RawIdentifierSyntax { text: "budget".to_owned(), span },
        parameters: Vec::new(),
        result_type: 0,
        body: RawFunctionBodySyntax {
            span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span,
                open_brace_span: span,
                statements: vec![0],
                close_brace_span: span,
            }],
            statements: vec![RawStatementSyntax {
                span,
                kind: RawStatementKind::Return {
                    keyword_span: span,
                    value: u32::try_from(child_count).expect("fixture root id"),
                    semicolon_span: span,
                },
            }],
            expressions,
        },
    }
}

fn nested_control_value_fixture(result_count: usize) -> RawFunctionSyntax {
    assert!(result_count >= 3);
    let mut function = derived_value_fixture(result_count - 2);
    let span = function.span;
    let result = u32::try_from(function.body.expressions.len() - 1).expect("result expression id");
    let if_condition = u32::try_from(function.body.expressions.len()).expect("if condition id");
    function.body.expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: true },
    });
    let while_condition =
        u32::try_from(function.body.expressions.len()).expect("while condition id");
    function.body.expressions.push(RawExpressionSyntax {
        span,
        kind: zryna_syntax::v4::RawExpressionKind::BoolLiteral { value: false },
    });
    function.body.blocks = vec![
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: vec![0, 3],
            close_brace_span: span,
        },
        RawBlockSyntax { span, open_brace_span: span, statements: vec![1], close_brace_span: span },
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: Vec::new(),
            close_brace_span: span,
        },
        RawBlockSyntax { span, open_brace_span: span, statements: vec![2], close_brace_span: span },
        RawBlockSyntax {
            span,
            open_brace_span: span,
            statements: Vec::new(),
            close_brace_span: span,
        },
    ];
    function.body.statements = vec![
        RawStatementSyntax { span, kind: RawStatementKind::Block { block: 1 } },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::If {
                keyword_span: span,
                open_paren_span: span,
                condition: if_condition,
                close_paren_span: span,
                then_block: 2,
                else_clause: Some(zryna_syntax::v4::RawElseSyntax { keyword_span: span, block: 3 }),
            },
        },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::While {
                keyword_span: span,
                open_paren_span: span,
                condition: while_condition,
                close_paren_span: span,
                body_block: 4,
            },
        },
        RawStatementSyntax {
            span,
            kind: RawStatementKind::Return {
                keyword_span: span,
                value: result,
                semicolon_span: span,
            },
        },
    ];
    function
}

#[allow(clippy::too_many_lines)]
fn authenticated_value_budget_fixture(with_parameter: bool) -> (String, RawProjectSyntaxSnapshot) {
    fn offset(text: &str) -> u32 {
        u32::try_from(text.len()).expect("fixture offset")
    }
    fn fixed_array_type(text: &mut String, types: &mut Vec<RawTypeSyntax>, length: usize) -> u32 {
        let start = offset(text);
        text.push_str("FixedArray<");
        let element_start = offset(text);
        text.push_str("i32");
        let element_end = offset(text);
        let element = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: element_start, end: element_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file: 0,
                        start: element_start,
                        end: element_end,
                    },
                },
            },
        });
        let comma = offset(text);
        text.push_str(", ");
        let length_start = offset(text);
        let spelling = length.to_string();
        text.push_str(&spelling);
        let length_end = offset(text);
        let greater = offset(text);
        text.push('>');
        let end = offset(text);
        let id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start, end },
            kind: RawTypeSyntaxKind::FixedArray {
                keyword_span: zryna_source::UntrustedSpan { file: 0, start, end: start + 10 },
                less_than_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: start + 10,
                    end: start + 11,
                },
                element,
                comma_span: zryna_source::UntrustedSpan { file: 0, start: comma, end: comma + 1 },
                length_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: length_start,
                    end: length_end,
                },
                length: u32::try_from(length).expect("array length"),
                length_spelling: spelling,
                greater_than_span: zryna_source::UntrustedSpan { file: 0, start: greater, end },
            },
        });
        id
    }

    let mut text = "function budget(".to_owned();
    let mut types = Vec::new();
    let mut parameters = Vec::new();
    if with_parameter {
        let parameter_start = offset(&text);
        text.push_str("x: ");
        let type_start = offset(&text);
        text.push_str("i32");
        let type_end = offset(&text);
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: type_start, end: type_end },
                },
            },
        });
        parameters.push(RawParameterSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: parameter_start, end: type_end },
            name: RawIdentifierSyntax {
                text: "x".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: parameter_start,
                    end: parameter_start + 1,
                },
            },
            type_syntax: 0,
        });
    }
    text.push_str("): ");
    let result_start = offset(&text);
    text.push_str("i32");
    let result_end = offset(&text);
    let result_type = u32::try_from(types.len()).expect("result type id");
    types.push(RawTypeSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax {
                text: "i32".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: result_start, end: result_end },
            },
        },
    });
    text.push_str(" { ");
    let body_start = result_end + 1;
    let mut expressions = Vec::with_capacity(zryna_syntax::v4::MAX_EXPRESSIONS_PER_FUNCTION);
    let mut statements = Vec::with_capacity(5);
    for (local, element_count) in [4_095_usize, 4_095, 4_095, 4_094].into_iter().enumerate() {
        let statement_start = offset(&text);
        text.push_str("const ");
        let name_start = offset(&text);
        let name = format!("a{local}");
        text.push_str(&name);
        let name_end = offset(&text);
        text.push_str(": ");
        let declared_type = fixed_array_type(&mut text, &mut types, element_count);
        text.push(' ');
        let equals = offset(&text);
        text.push_str("= ");
        let constructor_start = offset(&text);
        let constructor_type = fixed_array_type(&mut text, &mut types, element_count);
        let open_paren = offset(&text);
        text.push_str("([");
        let open_bracket = open_paren + 1;
        let first_element = u32::try_from(expressions.len()).expect("first element id");
        for index in 0..element_count {
            if index > 0 {
                text.push_str(", ");
            }
            let start = offset(&text);
            text.push('0');
            expressions.push(RawExpressionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start, end: start + 1 },
                kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
            });
        }
        let close_bracket = offset(&text);
        text.push_str("])");
        let close_paren = close_bracket + 1;
        let constructor_end = offset(&text);
        let initializer = u32::try_from(expressions.len()).expect("constructor id");
        expressions.push(RawExpressionSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: constructor_start,
                end: constructor_end,
            },
            kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
                type_syntax: constructor_type,
                open_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_paren,
                    end: open_paren + 1,
                },
                open_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: open_bracket,
                    end: open_bracket + 1,
                },
                elements: (first_element..initializer).collect(),
                close_bracket_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_bracket,
                    end: close_bracket + 1,
                },
                close_paren_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: close_paren,
                    end: close_paren + 1,
                },
            },
        });
        let semicolon = offset(&text);
        text.push_str("; ");
        statements.push(RawStatementSyntax {
            span: zryna_source::UntrustedSpan {
                file: 0,
                start: statement_start,
                end: semicolon + 1,
            },
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: statement_start,
                    end: statement_start + 5,
                },
                mutable: false,
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file: 0, start: name_start, end: name_end },
                },
                type_syntax: declared_type,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: equals,
                    end: equals + 1,
                },
                initializer,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: semicolon,
                    end: semicolon + 1,
                },
            },
        });
    }
    let return_start = offset(&text);
    text.push_str("return ");
    let value_start = offset(&text);
    text.push('0');
    let returned = u32::try_from(expressions.len()).expect("return value id");
    expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: value_start, end: value_start + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::I32Literal { spelling: "0".to_owned() },
    });
    let semicolon = offset(&text);
    text.push_str("; }");
    statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: return_start, end: semicolon + 1 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: return_start,
                end: return_start + 6,
            },
            value: returned,
            semicolon_span: zryna_source::UntrustedSpan {
                file: 0,
                start: semicolon,
                end: semicolon + 1,
            },
        },
    });
    let end = offset(&text);
    let body_end = end;
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: zryna_source::UntrustedSpan { file: 0, start: 0, end },
                export_span: None,
                function_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 8 },
                name: RawIdentifierSyntax {
                    text: "budget".to_owned(),
                    span: zryna_source::UntrustedSpan { file: 0, start: 9, end: 15 },
                },
                parameters,
                result_type,
                body: RawFunctionBodySyntax {
                    span: zryna_source::UntrustedSpan { file: 0, start: body_start, end: body_end },
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_end,
                        },
                        open_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_start,
                            end: body_start + 1,
                        },
                        statements: (0..u32::try_from(statements.len()).expect("statement ids"))
                            .collect(),
                        close_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_end - 1,
                            end: body_end,
                        },
                    }],
                    statements,
                    expressions,
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    (text, raw)
}

#[test]
fn derived_ir_value_budgets_are_exact_and_plus_one_is_rejected() {
    let per_function = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    assert_eq!(derived_value_count(&derived_value_fixture(per_function)), per_function);
    assert_eq!(derived_value_count(&derived_value_fixture(per_function + 1)), per_function + 1);

    assert_eq!(value_budget_violation(0, per_function), None);
    assert_eq!(value_budget_violation(0, per_function + 1), Some(ValueBudgetLimit::Function));
    let per_program = zryna_ir::data_ownership_v1::MAX_VALUES_PER_PROGRAM;
    assert_eq!(value_budget_violation(per_program - per_function, per_function), None);
    assert_eq!(
        value_budget_violation(per_program - per_function + 1, per_function),
        Some(ValueBudgetLimit::Program)
    );
    assert_eq!(value_budget_violation(usize::MAX, per_function), Some(ValueBudgetLimit::Program));
}

#[test]
fn nested_block_branch_and_loop_values_have_exact_checked_boundaries() {
    let exact = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let nested = nested_control_value_fixture(exact);
    assert_eq!(derived_value_count(&nested), exact);
    assert_eq!(value_budget_violation(0, derived_value_count(&nested)), None);

    let plus_one = nested_control_value_fixture(exact + 1);
    assert_eq!(derived_value_count(&plus_one), exact + 1);
    assert_eq!(
        value_budget_violation(0, derived_value_count(&plus_one)),
        Some(ValueBudgetLimit::Function)
    );
}

#[test]
fn terminal_semantic_budget_diagnostic_retains_the_triggering_source_span() {
    let sources = sources_for("x");
    let path = NormalizedSourcePath::new("src/main.zry").expect("normalized path");
    let file = sources.file_id(&path).expect("source file");
    let at = sources.span(file, 0, 1).expect("source span");
    let mut errors = Errors::new(&sources);
    for _ in 0..MAX_SEMANTIC_DIAGNOSTICS {
        errors.at("ZRYNA-M3201", at, "budget exceeded", "reduce the input");
    }
    let diagnostics = errors.finish();
    let terminal = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3202")
        .expect("terminal diagnostic");
    assert_eq!(terminal.primary_span(), Some(at));
}

#[test]
#[ignore = "authenticated exact/first-extra boundary runs in the full M3 preflight gate"]
fn authenticated_v4_derived_value_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_raw) = authenticated_value_budget_fixture(false);
    let exact_sources = sources_for(&exact_text);
    let exact_syntax = verify_snapshot(exact_raw, &exact_sources).expect("exact value-budget v4");
    let exact_input = pair_input(&exact_syntax, &exact_sources);
    let mut exact_errors = Errors::new(&exact_sources);
    semantic_preflight(exact_input, &mut exact_errors);
    assert!(exact_errors.finish().is_empty(), "exact value budget must pass preflight");

    let (plus_text, plus_raw) = authenticated_value_budget_fixture(true);
    let plus_sources = sources_for(&plus_text);
    let plus_syntax = verify_snapshot(plus_raw, &plus_sources).expect("plus-one value-budget v4");
    let mut plus_errors = Errors::new(&plus_sources);
    semantic_preflight(pair_input(&plus_syntax, &plus_sources), &mut plus_errors);
    let diagnostics = plus_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    let primary = diagnostics[0].primary_span().expect("function source span");
    assert_eq!(
        (primary.start(), primary.end()),
        (0, u32::try_from(plus_text.len()).expect("fixture length"))
    );
}
