use super::*;

impl Builder {
    pub(super) fn match_operand(&mut self, cloned: bool, nested: bool) -> u32 {
        if !nested {
            return self.matched(cloned);
        }
        let start = self.text.len();
        let type_syntax = self.payload_type(Payload::Array(2));
        let open_paren_span = self.text("(");
        let open_bracket_span = self.text("[");
        let literal_start = self.text.len();
        let spelling = "\"earlier\"";
        self.text(spelling);
        let first = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(literal_start),
            kind: RawExpressionKind::StringLiteral { spelling: spelling.into() },
        });
        self.text(", ");
        let second = self.matched(cloned);
        let close_bracket_span = self.text("]");
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: RawExpressionKind::FixedArrayConstruction {
                type_syntax,
                open_paren_span,
                open_bracket_span,
                elements: vec![first, second],
                close_bracket_span,
                close_paren_span,
            },
        });
        id
    }
}
