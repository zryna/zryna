use super::*;

pub(super) struct OwnedRootCleanupBoundaryFixture {
    pub(super) source: String,
    pub(super) raw: RawProjectSyntaxSnapshot,
    pub(super) return_span: zryna_source::UntrustedSpan,
    pub(super) source_expressions: usize,
    pub(super) source_statements: usize,
    pub(super) source_types: usize,
    pub(super) construction_operands: usize,
}

struct FixtureBuilder {
    source: String,
    types: Vec<RawTypeSyntax>,
    expressions: Vec<RawExpressionSyntax>,
    statements: Vec<RawStatementSyntax>,
}

impl FixtureBuilder {
    fn offset(&self) -> u32 {
        u32::try_from(self.source.len()).expect("fixture offset")
    }

    fn span(start: u32, end: u32) -> zryna_source::UntrustedSpan {
        zryna_source::UntrustedSpan { file: 0, start, end }
    }

    fn token(&mut self, text: &str) -> zryna_source::UntrustedSpan {
        let start = self.offset();
        self.source.push_str(text);
        Self::span(start, self.offset())
    }

    fn named_type(&mut self, name: &str) -> u32 {
        let span = self.token(name);
        let id = u32::try_from(self.types.len()).expect("type id");
        self.types.push(RawTypeSyntax {
            span,
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax { text: name.to_owned(), span },
            },
        });
        id
    }

    fn string_type(&mut self) -> u32 {
        let span = self.token("String");
        let id = u32::try_from(self.types.len()).expect("type id");
        self.types
            .push(RawTypeSyntax { span, kind: RawTypeSyntaxKind::String { keyword_span: span } });
        id
    }

    fn fixed_array_type(&mut self, length: usize) -> u32 {
        let start = self.offset();
        let keyword_span = self.token("FixedArray");
        let less_than_span = self.token("<");
        let element = self.string_type();
        let comma_span = self.token(",");
        self.token(" ");
        let spelling = length.to_string();
        let length_span = self.token(&spelling);
        let greater_than_span = self.token(">");
        let id = u32::try_from(self.types.len()).expect("type id");
        self.types.push(RawTypeSyntax {
            span: Self::span(start, self.offset()),
            kind: RawTypeSyntaxKind::FixedArray {
                keyword_span,
                less_than_span,
                element,
                comma_span,
                length_span,
                length_spelling: spelling,
                length: u32::try_from(length).expect("array length"),
                greater_than_span,
            },
        });
        id
    }

    fn borrow_root_type(&mut self) -> u32 {
        let start = self.offset();
        let keyword_span = self.token("Borrow");
        let less_than_span = self.token("<");
        let argument = self.named_type("Root");
        let greater_than_span = self.token(">");
        let id = u32::try_from(self.types.len()).expect("type id");
        self.types.push(RawTypeSyntax {
            span: Self::span(start, self.offset()),
            kind: RawTypeSyntaxKind::Borrow {
                keyword_span,
                less_than_span,
                argument,
                greater_than_span,
            },
        });
        id
    }

    fn reference(&mut self, name: &str) -> u32 {
        let span = self.token(name);
        let id = u32::try_from(self.expressions.len()).expect("expression id");
        self.expressions.push(RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: name.to_owned(), span },
            },
        });
        id
    }

    fn call_like(&mut self, keyword: &str, operand: u32, borrow: bool, start: u32) -> u32 {
        let keyword_span =
            Self::span(start, start + u32::try_from(keyword.len()).expect("keyword length"));
        let open_paren_span = Self::span(keyword_span.end, keyword_span.end + 1);
        let close_paren_span = self.token(")");
        let id = u32::try_from(self.expressions.len()).expect("expression id");
        let kind = if borrow {
            zryna_syntax::v4::RawExpressionKind::Borrow {
                keyword_span,
                open_paren_span,
                value: operand,
                close_paren_span,
            }
        } else {
            zryna_syntax::v4::RawExpressionKind::Clone {
                keyword_span,
                open_paren_span,
                value: operand,
                close_paren_span,
            }
        };
        self.expressions
            .push(RawExpressionSyntax { span: Self::span(start, close_paren_span.end), kind });
        id
    }

    fn local(&mut self, name: &str, type_syntax: u32, initializer: u32, start: u32) -> u32 {
        let semicolon_span = self.token(";");
        let id = u32::try_from(self.statements.len()).expect("statement id");
        let name_span = Self::span(
            start + 6,
            start + 6 + u32::try_from(name.len()).expect("local name length"),
        );
        let equals_start = self.types[type_syntax as usize].span.end + 1;
        self.statements.push(RawStatementSyntax {
            span: Self::span(start, semicolon_span.end),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: Self::span(start, start + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: name.to_owned(), span: name_span },
                type_syntax,
                equals_span: Self::span(equals_start, equals_start + 1),
                initializer,
                semicolon_span,
            },
        });
        id
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn owned_root_cleanup_boundary_fixture(
    left_length: usize,
    right_length: usize,
) -> OwnedRootCleanupBoundaryFixture {
    let mut b = FixtureBuilder {
        source: String::new(),
        types: Vec::new(),
        expressions: Vec::new(),
        statements: Vec::new(),
    };
    let declaration_start = b.offset();
    let interface_span = b.token("interface");
    b.token(" ");
    let declaration_name = b.token("Root");
    b.token(" ");
    let extends_span = b.token("extends");
    b.token(" ");
    let marker_span = b.token("ZrynaStruct");
    b.token(" ");
    let declaration_open = b.token("{");
    b.token("\n  ");
    let left_start = b.offset();
    let left_name = b.token("left");
    let left_colon = b.token(":");
    b.token(" ");
    let left_type = b.fixed_array_type(left_length);
    let left_semicolon = b.token(";");
    b.token("\n  ");
    let right_start = b.offset();
    let right_name = b.token("right");
    let right_colon = b.token(":");
    b.token(" ");
    let right_type = b.fixed_array_type(right_length);
    let right_semicolon = b.token(";");
    b.token("\n");
    let declaration_close = b.token("}");
    b.token("\n\n");
    let declaration = RawDataDeclaration {
        span: FixtureBuilder::span(declaration_start, declaration_close.end),
        export_span: None,
        kind: RawDataDeclarationKind::Struct {
            interface_span,
            name: RawIdentifierSyntax { text: "Root".to_owned(), span: declaration_name },
            extends_span,
            marker_span,
            open_brace_span: declaration_open,
            fields: vec![
                RawDataField {
                    span: FixtureBuilder::span(left_start, left_semicolon.end),
                    name: RawIdentifierSyntax { text: "left".to_owned(), span: left_name },
                    colon_span: left_colon,
                    type_syntax: left_type,
                    semicolon_span: left_semicolon,
                },
                RawDataField {
                    span: FixtureBuilder::span(right_start, right_semicolon.end),
                    name: RawIdentifierSyntax { text: "right".to_owned(), span: right_name },
                    colon_span: right_colon,
                    type_syntax: right_type,
                    semicolon_span: right_semicolon,
                },
            ],
            close_brace_span: declaration_close,
        },
    };

    let function_start = b.offset();
    let function_span = b.token("function");
    b.token(" ");
    let function_name = b.token("boundary");
    b.token("(): ");
    let result_type = b.named_type("Root");
    b.token(" ");
    let root_open = b.token("{");
    b.token("\n  ");

    let root_statement_start = b.offset();
    b.token("const root: ");
    let root_type = b.named_type("Root");
    b.token(" = ");
    let struct_start = b.offset();
    let struct_name = b.token("Root");
    let struct_open_paren = b.token("(");
    let struct_open_brace = b.token("{");
    b.token(" left: ");
    let left_field_name =
        FixtureBuilder::span(struct_open_brace.end + 1, struct_open_brace.end + 5);
    let left_field_colon = FixtureBuilder::span(left_field_name.end, left_field_name.end + 1);
    let left_expression_start = b.offset();
    let left_constructor_type = b.fixed_array_type(left_length);
    let left_array_open_paren = b.token("(");
    let left_array_open = b.token("[");
    let mut left_elements = Vec::with_capacity(left_length);
    for index in 0..left_length {
        if index > 0 {
            b.token(", ");
        }
        let span = b.token("\"\"");
        let id = u32::try_from(b.expressions.len()).expect("expression id");
        b.expressions.push(RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: "\"\"".to_owned(),
            },
        });
        left_elements.push(id);
    }
    let left_array_close = b.token("]");
    let left_array_close_paren = b.token(")");
    let left_value = u32::try_from(b.expressions.len()).expect("expression id");
    b.expressions.push(RawExpressionSyntax {
        span: FixtureBuilder::span(left_expression_start, left_array_close_paren.end),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: left_constructor_type,
            open_paren_span: left_array_open_paren,
            open_bracket_span: left_array_open,
            elements: left_elements,
            close_bracket_span: left_array_close,
            close_paren_span: left_array_close_paren,
        },
    });
    b.token(", right: ");
    let right_field_name =
        FixtureBuilder::span(left_array_close_paren.end + 2, left_array_close_paren.end + 7);
    let right_field_colon = FixtureBuilder::span(right_field_name.end, right_field_name.end + 1);
    let right_expression_start = b.offset();
    let right_constructor_type = b.fixed_array_type(right_length);
    let right_array_open_paren = b.token("(");
    let right_array_open = b.token("[");
    let mut right_elements = Vec::with_capacity(right_length);
    for index in 0..right_length {
        if index > 0 {
            b.token(", ");
        }
        let span = b.token("\"\"");
        let id = u32::try_from(b.expressions.len()).expect("expression id");
        b.expressions.push(RawExpressionSyntax {
            span,
            kind: zryna_syntax::v4::RawExpressionKind::StringLiteral {
                spelling: "\"\"".to_owned(),
            },
        });
        right_elements.push(id);
    }
    let right_array_close = b.token("]");
    let right_array_close_paren = b.token(")");
    let right_value = u32::try_from(b.expressions.len()).expect("expression id");
    b.expressions.push(RawExpressionSyntax {
        span: FixtureBuilder::span(right_expression_start, right_array_close_paren.end),
        kind: zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction {
            type_syntax: right_constructor_type,
            open_paren_span: right_array_open_paren,
            open_bracket_span: right_array_open,
            elements: right_elements,
            close_bracket_span: right_array_close,
            close_paren_span: right_array_close_paren,
        },
    });
    b.token(" }");
    let struct_close = b.token(")");
    let root_initializer = u32::try_from(b.expressions.len()).expect("expression id");
    b.expressions.push(RawExpressionSyntax {
        span: FixtureBuilder::span(struct_start, struct_close.end),
        kind: zryna_syntax::v4::RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax { text: "Root".to_owned(), span: struct_name },
            open_paren_span: struct_open_paren,
            open_brace_span: struct_open_brace,
            fields: vec![
                zryna_syntax::v4::RawFieldInitializer {
                    span: FixtureBuilder::span(left_field_name.start, left_array_close_paren.end),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "left".to_owned(),
                            span: left_field_name,
                        },
                        colon_span: left_field_colon,
                        value: left_value,
                    },
                },
                zryna_syntax::v4::RawFieldInitializer {
                    span: FixtureBuilder::span(right_field_name.start, right_array_close_paren.end),
                    kind: zryna_syntax::v4::RawFieldInitializerKind::Explicit {
                        name: RawIdentifierSyntax {
                            text: "right".to_owned(),
                            span: right_field_name,
                        },
                        colon_span: right_field_colon,
                        value: right_value,
                    },
                },
            ],
            close_brace_span: FixtureBuilder::span(struct_close.start - 1, struct_close.start),
            close_paren_span: struct_close,
        },
    });
    let root_statement = b.local("root", root_type, root_initializer, root_statement_start);
    b.token("\n  ");
    let nested_statement_start = b.offset();
    let nested_open = b.token("{");
    let nested_statement = u32::try_from(b.statements.len()).expect("statement id");
    b.statements
        .push(RawStatementSyntax { span: nested_open, kind: RawStatementKind::Block { block: 1 } });
    b.token("\n    ");
    let alias_statement_start = b.offset();
    b.token("const alias: ");
    let alias_type = b.borrow_root_type();
    b.token(" = ");
    let borrow_start = b.offset();
    b.token("borrow(");
    let root_reference = b.reference("root");
    let borrow_expression = b.call_like("borrow", root_reference, true, borrow_start);
    let alias_statement = b.local("alias", alias_type, borrow_expression, alias_statement_start);
    let mut clone_statements = Vec::with_capacity(510);
    for index in 0..510 {
        b.token("\n    ");
        let statement_start = b.offset();
        let name = format!("c{index}");
        b.token("const ");
        b.token(&name);
        b.token(": ");
        let clone_type = b.named_type("Root");
        b.token(" = ");
        let clone_start = b.offset();
        b.token("clone(");
        let alias_reference = b.reference("alias");
        let clone = b.call_like("clone", alias_reference, false, clone_start);
        clone_statements.push(b.local(&name, clone_type, clone, statement_start));
    }
    b.token("\n  ");
    let nested_close = b.token("}");
    b.statements[nested_statement as usize].span =
        FixtureBuilder::span(nested_statement_start, nested_close.end);
    b.token("\n  ");
    let return_start = b.offset();
    let return_keyword = b.token("return");
    b.token(" ");
    let returned = b.reference("root");
    let return_semicolon = b.token(";");
    let return_span = FixtureBuilder::span(return_start, return_semicolon.end);
    let return_statement = u32::try_from(b.statements.len()).expect("statement id");
    b.statements.push(RawStatementSyntax {
        span: return_span,
        kind: RawStatementKind::Return {
            keyword_span: return_keyword,
            value: returned,
            semicolon_span: return_semicolon,
        },
    });
    b.token("\n");
    let root_close = b.token("}");
    b.token("\n");

    let source_expressions = b.expressions.len();
    let source_statements = b.statements.len();
    let source_types = b.types.len();
    let construction_operands = b
        .expressions
        .iter()
        .map(|expression| match &expression.kind {
            zryna_syntax::v4::RawExpressionKind::FixedArrayConstruction { elements, .. } => {
                elements.len()
            }
            zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } => fields.len(),
            _ => 0,
        })
        .sum();
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: b.types,
            data_declarations: vec![declaration],
            functions: vec![RawFunctionSyntax {
                span: FixtureBuilder::span(function_start, root_close.end),
                export_span: None,
                function_span,
                name: RawIdentifierSyntax { text: "boundary".to_owned(), span: function_name },
                parameters: Vec::new(),
                result_type,
                body: RawFunctionBodySyntax {
                    span: FixtureBuilder::span(root_open.start, root_close.end),
                    root_block: 0,
                    blocks: vec![
                        RawBlockSyntax {
                            span: FixtureBuilder::span(root_open.start, root_close.end),
                            open_brace_span: root_open,
                            statements: vec![root_statement, nested_statement, return_statement],
                            close_brace_span: root_close,
                        },
                        RawBlockSyntax {
                            span: FixtureBuilder::span(nested_open.start, nested_close.end),
                            open_brace_span: nested_open,
                            statements: std::iter::once(alias_statement)
                                .chain(clone_statements)
                                .collect(),
                            close_brace_span: nested_close,
                        },
                    ],
                    statements: b.statements,
                    expressions: b.expressions,
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    OwnedRootCleanupBoundaryFixture {
        source: b.source,
        raw,
        return_span,
        source_expressions,
        source_statements,
        source_types,
        construction_operands,
    }
}
