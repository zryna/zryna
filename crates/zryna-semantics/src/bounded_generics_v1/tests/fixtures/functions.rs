//! Complete source and DTO construction for declaration fixture functions.

use super::*;

impl Unit {
    pub(in super::super) fn function(&mut self, text: &str, names: &[&str], exported: bool) {
        let export_span = self.exported(exported);
        let function_span = self.token("function");
        self.token(" ");
        let name = self.identifier(text);
        let type_parameters = self.parameters(names);
        self.token("(");
        let mut parameters = Vec::new();
        if let Some(ty) = names.first() {
            let name = self.identifier("value");
            self.token(": ");
            let type_syntax = self.annotation(ty);
            parameters.push(RawParameterSyntax {
                span: self
                    .span(name.span.start, self.raw.type_syntax[type_syntax as usize].span.end),
                name,
                type_syntax,
            });
        }
        self.token("): ");
        let result_type = self.annotation(names.first().copied().unwrap_or("i32"));
        self.token(" ");
        let open_brace_span = self.token("{");
        self.token(" ");
        let keyword_span = self.token("return");
        self.token(" ");
        let expression = if names.is_empty() {
            RawExpressionSyntax {
                span: self.token("1"),
                kind: RawExpressionKind::I32Literal { spelling: "1".into() },
            }
        } else {
            let name = self.identifier("value");
            RawExpressionSyntax { span: name.span, kind: RawExpressionKind::Reference { name } }
        };
        let semicolon_span = self.token(";");
        let statement = RawStatementSyntax {
            span: self.span(keyword_span.start, semicolon_span.end),
            kind: RawStatementKind::Return { keyword_span, value: 0, semicolon_span },
        };
        self.token(" ");
        let close_brace_span = self.token("}");
        let body_span = self.span(open_brace_span.start, close_brace_span.end);
        self.raw.functions.push(RawFunctionSyntax {
            span: self.span(export_span.unwrap_or(function_span).start, close_brace_span.end),
            export_span,
            function_span,
            name,
            type_parameters,
            parameters,
            result_type,
            body: RawFunctionBodySyntax {
                span: body_span,
                root_block: 0,
                blocks: vec![RawBlockSyntax {
                    span: body_span,
                    open_brace_span,
                    statements: vec![0],
                    close_brace_span,
                }],
                statements: vec![statement],
                expressions: vec![expression],
            },
        });
        self.token("\n");
    }
}
