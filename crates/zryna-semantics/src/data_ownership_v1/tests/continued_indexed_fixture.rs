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

    fn fresh_container_choice(
        &mut self,
        vector: bool,
        owned: bool,
        mismatched: bool,
    ) -> RawDataDeclaration {
        let start = self.text.len();
        let interface_span = self.text("interface");
        self.text(" ");
        let name = self.name("Choice");
        self.text(" ");
        let extends_span = self.text("extends");
        self.text(" ");
        let marker_span = self.text("ZrynaEnum");
        self.text(" ");
        let open_brace_span = self.text("{");
        let mut variants = Vec::new();
        for text in ["left", "right"] {
            self.text(" ");
            let variant_start = self.text.len();
            let name = self.name(text);
            let colon_span = self.text(":");
            self.text(" ");
            let payload_owned = if mismatched && text == "right" { !owned } else { owned };
            let payload_type = Some(self.continued_container(vector, payload_owned, 1));
            let semicolon_span = self.text(";");
            variants.push(RawEnumVariant {
                span: self.span(variant_start),
                name,
                colon_span,
                payload_type,
                none_span: None,
                semicolon_span,
            });
        }
        self.text(" ");
        let close_brace_span = self.text("}");
        RawDataDeclaration {
            span: self.span(start),
            export_span: None,
            kind: RawDataDeclarationKind::Enum {
                interface_span,
                name,
                extends_span,
                marker_span,
                open_brace_span,
                close_brace_span,
                variants,
            },
        }
    }

    fn fresh_match_target(&mut self, owned: bool, clone_owned: bool) -> u32 {
        let clone = (owned && clone_owned).then(|| {
            let start = self.text.len();
            let keyword_span = self.text("clone");
            let open_paren_span = self.text("(");
            (start, keyword_span, open_paren_span)
        });
        let indexed_start = self.text.len();
        let base = self.matched(false);
        let open_bracket_span = self.text("[");
        let index = self.continued_index_call();
        let close_bracket_span = self.text("]");
        let indexed = u32::try_from(self.expressions.len()).expect("bounded fresh Match fixture");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(indexed_start),
            kind: RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
        });
        if !owned || !clone_owned {
            return indexed;
        }
        let close_paren_span = self.text(")");
        let (start, keyword_span, open_paren_span) = clone.expect("owned clone prefix");
        let clone = u32::try_from(self.expressions.len()).expect("bounded fresh Match fixture");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::Clone {
                keyword_span,
                open_paren_span,
                value: indexed,
                close_paren_span,
            },
        });
        clone
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

pub(in crate::data_ownership_v1) fn fresh_match_base_fixture(
    vector: bool,
    owned: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    let mode = if owned { FreshMode::OwnedExplicit } else { FreshMode::Copy };
    fresh_match_base_build(container, mode)
}

pub(in crate::data_ownership_v1) fn fresh_match_base_implicit_read_fixture(
    vector: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    fresh_match_base_build(container, FreshMode::OwnedImplicit)
}

pub(in crate::data_ownership_v1) fn fresh_match_base_mismatch_fixture(
    vector: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    fresh_match_base_build(container, FreshMode::Mismatched)
}

#[derive(Clone, Copy)]
enum FreshContainer {
    FixedArray,
    Vec,
}

#[derive(Clone, Copy)]
enum FreshMode {
    Copy,
    OwnedExplicit,
    OwnedImplicit,
    Mismatched,
}

fn fresh_match_base_build(
    container: FreshContainer,
    mode: FreshMode,
) -> (String, RawProjectSyntaxSnapshot) {
    let vector = matches!(container, FreshContainer::Vec);
    let (owned, clone_owned, mismatched) = match mode {
        FreshMode::Copy => (false, true, false),
        FreshMode::OwnedExplicit => (true, true, false),
        FreshMode::OwnedImplicit => (true, false, false),
        FreshMode::Mismatched => (false, true, true),
    };
    let mut builder = Builder::default();
    let declaration = builder.fresh_container_choice(vector, owned, mismatched);
    builder.text("\n");
    let start = builder.text.len();
    let function_span = builder.text("function");
    builder.text(" ");
    let name = builder.name("observe");
    builder.text("(");
    let mut parameters = Vec::new();
    for (ordinal, text) in ["source", "offset"].into_iter().enumerate() {
        if ordinal > 0 {
            builder.text(", ");
        }
        let parameter_start = builder.text.len();
        let name = builder.name(text);
        builder.text(": ");
        let type_syntax =
            if ordinal == 0 { builder.named_type("Choice") } else { builder.named_type("i32") };
        parameters.push(RawParameterSyntax {
            span: builder.span(parameter_start),
            name,
            type_syntax,
        });
    }
    builder.text("): ");
    let result_type = builder.payload_type(if owned { Payload::String } else { Payload::I32 });
    builder.text(" ");
    let body_start = builder.text.len();
    let open_brace_span = builder.text("{");
    builder.text(" ");
    let statement_start = builder.text.len();
    let keyword_span = builder.text("return");
    builder.text(" ");
    let value = builder.fresh_match_target(owned, clone_owned);
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: builder.span(statement_start),
        kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
    });
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
                statements: vec![0],
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    };
    let callee = builder.continued_callee();
    finish(builder, vec![declaration], vec![function, callee])
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
