use super::*;

impl Builder {
    pub(super) fn indexed_container_type(&mut self, kind: OperandKind) -> u32 {
        let start = self.text.len();
        let vector = matches!(kind, OperandKind::IndexedVec | OperandKind::IndexedOwnedVec);
        let keyword_span = self.text(if vector { "Vec" } else { "FixedArray" });
        let less_than_span = self.text("<");
        let element =
            if matches!(kind, OperandKind::IndexedOwnedArray | OperandKind::IndexedOwnedVec) {
                self.ty(true)
            } else {
                self.named_type("i32")
            };
        let kind = if vector {
            let greater_than_span = self.text(">");
            RawTypeSyntaxKind::Vec {
                keyword_span,
                less_than_span,
                argument: element,
                greater_than_span,
            }
        } else {
            let comma_span = self.text(",");
            self.text(" ");
            let length_span = self.text("2");
            let greater_than_span = self.text(">");
            RawTypeSyntaxKind::FixedArray {
                keyword_span,
                less_than_span,
                element,
                comma_span,
                length_span,
                length_spelling: "2".into(),
                length: 2,
                greater_than_span,
            }
        };
        let id = u32::try_from(self.types.len()).expect("container type count");
        self.types.push(RawTypeSyntax { span: self.span(start), kind });
        id
    }

    pub(super) fn indexed_match_operand(&mut self, kind: OperandKind) -> u32 {
        let start = self.text.len();
        let clone = if matches!(kind, OperandKind::IndexedOwnedArray | OperandKind::IndexedOwnedVec)
        {
            let keyword = self.text("clone");
            let open = self.text("(");
            Some((keyword, open))
        } else {
            None
        };
        let indexed_start = self.text.len();
        let base = self.reference("items");
        let open_bracket_span = self.text("[");
        let index = self.matched(false);
        let close_bracket_span = self.text("]");
        let id = u32::try_from(self.expressions.len()).expect("index expression count");
        self.expressions.push(RawExpressionSyntax {
            span: self.span(indexed_start),
            kind: RawExpressionKind::Index { base, open_bracket_span, index, close_bracket_span },
        });
        if let Some((keyword_span, open_paren_span)) = clone {
            let close_paren_span = self.text(")");
            let result = u32::try_from(self.expressions.len()).expect("clone expression count");
            self.expressions.push(RawExpressionSyntax {
                span: self.span(start),
                kind: RawExpressionKind::Clone {
                    keyword_span,
                    open_paren_span,
                    value: id,
                    close_paren_span,
                },
            });
            result
        } else {
            id
        }
    }
}
