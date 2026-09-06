use super::*;

impl Builder {
    fn continued_container(&mut self, vector: bool, owned: bool, depth: usize) -> u32 {
        if depth == 0 {
            return self.payload_type(if owned { Payload::String } else { Payload::I32 });
        }
        let start = self.text.len();
        let keyword_span = self.text(if vector { "Vec" } else { "FixedArray" });
        let less_than_span = self.text("<");
        let element = self.continued_container(vector, owned, depth - 1);
        let kind = if vector {
            RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element,
                greater_than_span: self.text(">"),
            }
        } else {
            let comma_span = self.text(",");
            self.text(" ");
            let length_span = self.text("2");
            RawTypeSyntaxKind::FixedArray {
                keyword_span,
                less_than_span,
                element,
                comma_span,
                length_span,
                length_spelling: "2".into(),
                length: 2,
                greater_than_span: self.text(">"),
            }
        };
        let id = u32::try_from(self.types.len()).expect("bounded authenticated fixture");
        self.types.push(RawTypeSyntax { span: self.span(start), kind });
        id
    }

    fn continued_target(&mut self, write: bool) -> u32 {
        let start = self.text.len();
        let mut base = self.reference("items");
        for level in 0..if write { 1 } else { 2 } {
            let open_bracket_span = self.text("[");
            let index = if level == 0 { self.continued_index_call() } else { self.matched(false) };
            let close_bracket_span = self.text("]");
            let id = u32::try_from(self.expressions.len()).expect("bounded authenticated fixture");
            self.expressions.push(RawExpressionSyntax {
                span: self.span(start),
                kind: RawExpressionKind::Index {
                    base,
                    open_bracket_span,
                    index,
                    close_bracket_span,
                },
            });
            base = id;
        }
        base
    }

    fn continued_index_call(&mut self) -> u32 {
        let start = self.text.len();
        let callee = self.name("indexValue");
        let open_paren_span = self.text("(");
        let offset = self.reference("offset");
        self.text(", ");
        let literal_start = self.text.len();
        self.text("\"index\"");
        let literal = u32::try_from(self.expressions.len()).expect("bounded authenticated fixture");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(literal_start),
            kind: RawExpressionKind::StringLiteral { spelling: "\"index\"".into() },
        });
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("bounded authenticated fixture");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::Call {
                callee,
                open_paren_span,
                arguments: vec![offset, literal],
                close_paren_span,
            },
        });
        id
    }

    fn continued_callee(&mut self) -> RawFunctionSyntax {
        self.text("\n");
        let start = self.text.len();
        let function_span = self.text("function");
        self.text(" ");
        let name = self.name("indexValue");
        self.text("(");
        let mut parameters = Vec::new();
        for (name, owned) in [("value", false), ("held", true)] {
            if !parameters.is_empty() {
                self.text(", ");
            }
            let start = self.text.len();
            let name = self.name(name);
            self.text(": ");
            let type_syntax = self.payload_type(if owned { Payload::String } else { Payload::I32 });
            parameters.push(RawParameterSyntax { span: self.span(start), name, type_syntax });
        }
        self.text("): ");
        let result_type = self.payload_type(Payload::I32);
        self.text(" ");
        let root_block = self.block(&[Statement::Return("value")]);
        RawFunctionSyntax {
            span: self.span(start),
            export_span: None,
            function_span,
            name,
            parameters,
            result_type,
            body: RawFunctionBodySyntax {
                span: self.blocks[root_block as usize].span,
                root_block,
                blocks: std::mem::take(&mut self.blocks),
                statements: std::mem::take(&mut self.statements),
                expressions: std::mem::take(&mut self.expressions),
            },
        }
    }
}

pub(in crate::data_ownership_v1) fn fixture(
    vector: bool,
    owned: bool,
    write: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build((vector, owned, write), (owned, true))
}

pub(in crate::data_ownership_v1) fn rejection_fixture(
    vector: bool,
    immutable: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    build((vector, true, true), (immutable, !immutable))
}

fn build(shape: (bool, bool, bool), options: (bool, bool)) -> (String, RawProjectSyntaxSnapshot) {
    let (vector, owned, write) = shape;
    let (rhs_owned, _) = options;
    let mut builder = Builder::default();
    let declaration =
        builder.enumeration(if rhs_owned && write { Payload::String } else { Payload::I32 });
    builder.text("\n");
    let start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("operate");
    builder.text("(");
    let mut parameters = Vec::new();
    for (ordinal, name) in
        ["source", if write { "incoming" } else { "items" }, "offset"].into_iter().enumerate()
    {
        if ordinal > 0 {
            builder.text(", ");
        }
        let start = builder.text.len();
        let name = builder.name(name);
        builder.text(": ");
        let type_syntax = match ordinal {
            0 => builder.named_type("Choice"),
            1 => builder.continued_container(vector, owned, if write { 1 } else { 2 }),
            _ => builder.named_type("i32"),
        };
        parameters.push(RawParameterSyntax { span: builder.span(start), name, type_syntax });
    }
    builder.text("): ");
    let result_type = builder.continued_container(vector, owned, usize::from(write));
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    builder.continued_statements(shape, options);
    builder.text(" ");
    let close_brace_span = builder.text("}");
    let body_span = builder.span(body_start);
    let function = RawFunctionSyntax {
        span: builder.span(start),
        export_span: None,
        function_span,
        name,
        parameters,
        result_type,
        body: RawFunctionBodySyntax {
            span: body_span,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body_span,
                open_brace_span,
                close_brace_span,
                statements: (0..u32::try_from(builder.statements.len())
                    .expect("bounded authenticated fixture"))
                    .collect(),
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    };
    let callee = builder.continued_callee();
    finish(builder, vec![declaration], vec![function, callee])
}

impl Builder {
    fn continued_statements(&mut self, shape: (bool, bool, bool), options: (bool, bool)) {
        let (vector, owned, write) = shape;
        let (rhs_owned, mutable) = options;
        if write {
            let start = self.text.len();
            let keyword_span = self.text(if mutable { "let" } else { "const" });
            self.text(" ");
            let name = self.name("items");
            self.text(": ");
            let type_syntax = self.continued_container(vector, owned, 1);
            self.text(" ");
            let equals_span = self.text("=");
            self.text(" ");
            let initializer = self.reference("incoming");
            let semicolon_span = self.text(";");
            self.statements.push(RawStatementSyntax {
                span: self.span(start),
                kind: RawStatementKind::LocalDeclaration {
                    keyword_span,
                    mutable,
                    name,
                    type_syntax,
                    equals_span,
                    initializer,
                    semicolon_span,
                },
            });
            self.text(" ");
            let start = self.text.len();
            let target = self.continued_target(true);
            self.text(" ");
            let equals_span = self.text("=");
            self.text(" ");
            let value = self.matched(rhs_owned);
            let semicolon_span = self.text(";");
            self.statements.push(RawStatementSyntax {
                span: self.span(start),
                kind: RawStatementKind::Assignment { target, equals_span, value, semicolon_span },
            });
            self.text(" ");
            self.statement(&Statement::Return("items"));
        } else {
            let start = self.text.len();
            let keyword_span = self.text("return");
            self.text(" ");
            let clone = owned.then(|| (self.text("clone"), self.text("(")));
            let indexed_start = self.text.len();
            let mut value = self.continued_target(false);
            if let Some((keyword_span, open_paren_span)) = clone {
                let close_paren_span = self.text(")");
                let id =
                    u32::try_from(self.expressions.len()).expect("bounded authenticated fixture");
                self.expressions.push(RawExpressionSyntax {
                    span: UntrustedSpan {
                        start: u32::try_from(indexed_start - 6)
                            .expect("bounded authenticated fixture"),
                        ..self.span(indexed_start)
                    },
                    kind: RawExpressionKind::Clone {
                        keyword_span,
                        open_paren_span,
                        value,
                        close_paren_span,
                    },
                });
                value = id;
            }
            let semicolon_span = self.text(";");
            self.statements.push(RawStatementSyntax {
                span: self.span(start),
                kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
            });
        }
    }
}
