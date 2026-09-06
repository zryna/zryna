use super::*;

impl Builder {
    pub(super) fn match_operand(&mut self, cloned: bool, operand: OperandKind) -> u32 {
        if matches!(
            operand,
            OperandKind::IndexedArray
                | OperandKind::IndexedVec
                | OperandKind::IndexedOwnedArray
                | OperandKind::IndexedOwnedVec
                | OperandKind::IndexedNestedArray
                | OperandKind::IndexedNestedVec
        ) {
            return self.indexed_match_operand(operand);
        }
        if matches!(
            operand,
            OperandKind::StringClone | OperandKind::StringConcat | OperandKind::StringNamed
        ) {
            return self.string_match_operand(operand, cloned);
        }
        if matches!(operand, OperandKind::Plain) {
            return self.matched(cloned);
        }
        let start = self.text.len();
        let call = operand.call();
        let callee = call.then(|| self.name("collect"));
        let type_syntax = if call {
            None
        } else {
            Some(self.payload_type(if matches!(operand, OperandKind::Vec) {
                Payload::Vec
            } else {
                Payload::Array(2)
            }))
        };
        let open_paren_span = self.text("(");
        let open_bracket_span = if call { open_paren_span } else { self.text("[") };
        let loan = operand.formal().map(|_| {
            let value = self.reference("loan");
            self.text(", ");
            value
        });
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
        let close_bracket_span = if call { open_paren_span } else { self.text("]") };
        let close_paren_span = self.text(")");
        let id = u32::try_from(self.expressions.len()).expect("expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(start),
            kind: if let Some(callee) = callee {
                RawExpressionKind::Call {
                    callee,
                    open_paren_span,
                    arguments: loan.into_iter().chain([first, second]).collect(),
                    close_paren_span,
                }
            } else if matches!(operand, OperandKind::Vec) {
                RawExpressionKind::VecConstruction {
                    type_syntax: type_syntax.expect("Vec type"),
                    open_paren_span,
                    open_bracket_span,
                    elements: vec![first, second],
                    close_bracket_span,
                    close_paren_span,
                }
            } else {
                RawExpressionKind::FixedArrayConstruction {
                    type_syntax: type_syntax.expect("array type"),
                    open_paren_span,
                    open_bracket_span,
                    elements: vec![first, second],
                    close_bracket_span,
                    close_paren_span,
                }
            },
        });
        id
    }
}
