use super::*;

impl Builder {
    pub(super) fn match_callee(&mut self) -> RawFunctionSyntax {
        self.text("\n");
        let start = self.text.len();
        let function_span = self.text("function");
        self.text(" ");
        let name = self.name("collect");
        self.text("(");
        let mut parameters = Vec::new();
        for name in ["first", "second"] {
            if !parameters.is_empty() {
                self.text(", ");
            }
            let start = self.text.len();
            let name = self.name(name);
            self.text(": ");
            let type_syntax = self.ty(true);
            parameters.push(RawParameterSyntax { span: self.span(start), name, type_syntax });
        }
        self.text("): ");
        let result_type = self.payload_type(Payload::Array(2));
        self.text(" ");
        let body_start = self.text.len();
        let open_brace_span = self.text("{");
        self.text(" ");
        let statement_start = self.text.len();
        let keyword_span = self.text("return");
        self.text(" ");
        let expression_start = self.text.len();
        let type_syntax = self.payload_type(Payload::Array(2));
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let first = self.reference("first");
        self.text(", ");
        let second = self.reference("second");
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        let value = u32::try_from(self.expressions.len()).expect("callee expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(expression_start),
            kind: RawExpressionKind::FixedArrayConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements: vec![first, second],
                close_bracket_span,
                close_paren_span,
            },
        });
        let semicolon_span = self.text(";");
        let statement = RawStatementSyntax {
            span: self.span(statement_start),
            kind: RawStatementKind::Return { keyword_span, value, semicolon_span },
        };
        self.text(" ");
        let close_brace_span = self.text("}");
        let body_span = self.span(body_start);
        RawFunctionSyntax {
            span: self.span(start),
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
                statements: vec![statement],
                expressions: std::mem::take(&mut self.expressions),
            },
        }
    }
}
