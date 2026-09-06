use super::*;

fn finish_observation_statement(
    builder: &mut Builder,
    start: usize,
    keyword: Option<UntrustedSpan>,
    value: u32,
) {
    if let Some(keyword_span) = keyword {
        let semicolon_span = builder.text(";");
        builder.statements.push(RawStatementSyntax {
            span: builder.span(start),
            kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
        });
        return;
    }
    builder.text(" ");
    let equals_span = builder.text("=");
    builder.text(" ");
    let literal_start = builder.text.len();
    builder.text("0");
    let literal = u32::try_from(builder.expressions.len()).expect("assignment literal");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(literal_start),
        kind: RawExpressionKind::I32Literal { spelling: "0".into() },
    });
    let semicolon_span = builder.text(";");
    builder.statements.push(RawStatementSyntax {
        span: builder.span(start),
        kind: RawStatementKind::Assignment {
            target: value,
            equals_span,
            value: literal,
            semicolon_span,
        },
    });
    builder.text(" ");
    let start = builder.text.len();
    let keyword = builder.text("return");
    builder.text(" ");
    let literal_start = builder.text.len();
    builder.text("0");
    let literal = u32::try_from(builder.expressions.len()).expect("return literal");
    builder.expressions.push(RawExpressionSyntax {
        span: builder.span(literal_start),
        kind: RawExpressionKind::I32Literal { spelling: "0".into() },
    });
    finish_observation_statement(builder, start, Some(keyword), literal);
}

pub(in crate::data_ownership_v1) fn assignment_fixture() -> (String, RawProjectSyntaxSnapshot) {
    fresh_match_base_build(FreshContainer::FixedArray, FreshMode::Assignment, FreshPath::Literal)
}

impl Builder {
    fn fresh_container_choice(
        &mut self,
        vector: bool,
        owned: bool,
        mismatched: bool,
        depth: usize,
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
            let payload_type = Some(self.continued_container(vector, payload_owned, depth));
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

    fn fresh_match_target(&mut self, owned: bool, clone_owned: bool, path: FreshPath) -> u32 {
        let clone = (owned && clone_owned).then(|| {
            let start = self.text.len();
            let keyword_span = self.text("clone");
            let open_paren_span = self.text("(");
            (start, keyword_span, open_paren_span)
        });
        let indexed_start = self.text.len();
        let mut indexed = self.matched(false);
        for ordinal in 0..path.depth() {
            let open_bracket_span = self.text("[");
            let index = if matches!(path, FreshPath::Call)
                || matches!(path, FreshPath::NestedCall) && ordinal == 1
            {
                self.continued_index_call()
            } else {
                let start = self.text.len();
                self.text("0");
                let id = u32::try_from(self.expressions.len()).expect("literal identity");
                self.expressions.push(RawExpressionSyntax {
                    span: self.span(start),
                    kind: RawExpressionKind::I32Literal { spelling: "0".into() },
                });
                id
            };
            let close_bracket_span = self.text("]");
            let id = u32::try_from(self.expressions.len()).expect("indexed identity");
            self.expressions.push(RawExpressionSyntax {
                span: self.span(indexed_start),
                kind: RawExpressionKind::Index {
                    base: indexed,
                    open_bracket_span,
                    index,
                    close_bracket_span,
                },
            });
            indexed = id;
        }
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

pub(in crate::data_ownership_v1) fn fresh_match_base_fixture(
    vector: bool,
    owned: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    let mode = if owned { FreshMode::OwnedExplicit } else { FreshMode::Copy };
    fresh_match_base_build(container, mode, FreshPath::Call)
}

pub(in crate::data_ownership_v1) fn fresh_match_base_implicit_read_fixture(
    vector: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    fresh_match_base_build(container, FreshMode::OwnedImplicit, FreshPath::Call)
}

pub(in crate::data_ownership_v1) fn fresh_match_base_mismatch_fixture(
    vector: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let container = if vector { FreshContainer::Vec } else { FreshContainer::FixedArray };
    fresh_match_base_build(container, FreshMode::Mismatched, FreshPath::Call)
}

#[derive(Clone, Copy)]
enum FreshContainer {
    FixedArray,
    Vec,
}

#[derive(Clone, Copy)]
enum FreshMode {
    Copy,
    Assignment,
    OwnedExplicit,
    OwnedImplicit,
    Mismatched,
}

fn fresh_match_base_build(
    container: FreshContainer,
    mode: FreshMode,
    path: FreshPath,
) -> (String, RawProjectSyntaxSnapshot) {
    let vector = matches!(container, FreshContainer::Vec);
    let (owned, clone_owned, mismatched) = match mode {
        FreshMode::Copy | FreshMode::Assignment => (false, true, false),
        FreshMode::OwnedExplicit => (true, true, false),
        FreshMode::OwnedImplicit => (true, false, false),
        FreshMode::Mismatched => (false, true, true),
    };
    let mut builder = Builder::default();
    let declaration = builder.fresh_container_choice(vector, owned, mismatched, path.depth());
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
    let keyword_span = if matches!(mode, FreshMode::Assignment) {
        None
    } else {
        let keyword = builder.text("return");
        builder.text(" ");
        Some(keyword)
    };
    let value = builder.fresh_match_target(owned, clone_owned, path);
    finish_observation_statement(&mut builder, statement_start, keyword_span, value);
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
                statements: (0..u32::try_from(builder.statements.len()).expect("bounded body"))
                    .collect(),
            }],
            statements: std::mem::take(&mut builder.statements),
            expressions: std::mem::take(&mut builder.expressions),
        },
    };
    let callee = builder.continued_callee();
    finish(builder, vec![declaration], vec![function, callee])
}
#[derive(Clone, Copy)]
enum FreshPath {
    Call,
    Literal,
    NestedLiteral,
    NestedCall,
}

impl FreshPath {
    const fn depth(self) -> usize {
        match self {
            Self::Call | Self::Literal => 1,
            Self::NestedLiteral | Self::NestedCall => 2,
        }
    }
}

pub(in crate::data_ownership_v1) fn literal_fixture(
    owned: bool,
    shape: usize,
) -> (String, RawProjectSyntaxSnapshot) {
    let path = match shape {
        0 => FreshPath::Literal,
        1 => FreshPath::NestedLiteral,
        2 => FreshPath::NestedCall,
        _ => unreachable!("three literal-prefix shapes"),
    };
    fresh_match_base_build(
        FreshContainer::FixedArray,
        if owned { FreshMode::OwnedExplicit } else { FreshMode::Copy },
        path,
    )
}
