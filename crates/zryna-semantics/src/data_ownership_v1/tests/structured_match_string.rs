use super::*;

impl Builder {
    pub(super) fn string_match_operand(&mut self, mode: OperandKind, cloned: bool) -> u32 {
        let start = self.text.len();
        let callee =
            self.name(if matches!(mode, OperandKind::StringClone) { "clone" } else { "concat" });
        let open_paren_span = self.text("(");
        let mut arguments = Vec::new();
        if matches!(mode, OperandKind::StringNamed) {
            arguments.push(self.reference("saved"));
            self.text(", ");
        } else if matches!(mode, OperandKind::StringConcat) {
            let start = self.text.len();
            let spelling = "\"earlier\"";
            self.text(spelling);
            let id = u32::try_from(self.expressions.len()).expect("literal expression");
            self.expressions.push(RawExpressionSyntax {
                span: self.span(start),
                kind: RawExpressionKind::StringLiteral { spelling: spelling.into() },
            });
            arguments.push(id);
            self.text(", ");
        }
        arguments.push(self.matched(cloned));
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("String expression");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: if matches!(mode, OperandKind::StringClone) {
                RawExpressionKind::Clone {
                    keyword_span: callee.span,
                    open_paren_span,
                    value: arguments[0],
                    close_paren_span,
                }
            } else {
                RawExpressionKind::Call { callee, open_paren_span, arguments, close_paren_span }
            },
        });
        id
    }
}
