use super::*;

impl Builder {
    pub(super) fn formal_parameter(
        &mut self,
        parameters: &mut Vec<RawParameterSyntax>,
        exclusive: bool,
    ) {
        if !parameters.is_empty() {
            self.text(", ");
        }
        let start = self.text.len();
        let name = self.name("loan");
        self.text(": ");
        let type_syntax = self.formal_type(exclusive);
        parameters.push(RawParameterSyntax { span: self.span(start), name, type_syntax });
    }

    fn formal_type(&mut self, exclusive: bool) -> u32 {
        let type_start = self.text.len();
        let keyword_span = self.text(if exclusive { "BorrowMut" } else { "Borrow" });
        let less_than_span = self.text("<");
        let argument = self.ty(true);
        let greater_than_span = self.text(">");
        let type_syntax = u32::try_from(self.types.len()).expect("formal type count");
        self.types.push(RawTypeSyntax {
            span: self.span(type_start),
            kind: if exclusive {
                RawTypeSyntaxKind::BorrowMut {
                    keyword_span,
                    less_than_span,
                    argument,
                    greater_than_span,
                }
            } else {
                RawTypeSyntaxKind::Borrow {
                    keyword_span,
                    less_than_span,
                    argument,
                    greater_than_span,
                }
            },
        });
        type_syntax
    }

    pub(super) fn lexical_declaration(&mut self) {
        let start = self.text.len();
        let keyword_span = self.text("const");
        self.text(" ");
        let name = self.name("loan");
        self.text(": ");
        let type_syntax = self.formal_type(false);
        self.text(" ");
        let equals_span = self.text("=");
        self.text(" ");
        let expression_start = self.text.len();
        let borrow_span = self.text("borrow");
        let open_paren_span = self.text("(");
        let value = self.reference("container");
        let close_paren_span = self.text(")");
        let initializer = u32::try_from(self.expressions.len()).expect("borrow expression");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(expression_start),
            kind: RawExpressionKind::Borrow {
                keyword_span: borrow_span,
                open_paren_span,
                value,
                close_paren_span,
            },
        });
        let semicolon_span = self.text(";");
        self.statements.push(RawStatementSyntax {
            span: self.span(start),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span,
                mutable: false,
                name,
                type_syntax,
                equals_span,
                initializer,
                semicolon_span,
            },
        });
    }
}
