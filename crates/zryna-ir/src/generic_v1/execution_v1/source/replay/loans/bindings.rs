//! Lexical loans and RHS-first replacement retain real source regions and identities.
use super::{Body, Expr, Failure, UntrustedSpan, charge, raw, reject};
impl<'s> Body<'_, 's> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn local_loan(
        &mut self,
        name: &'s str,
        expected: (&[u8], bool),
        mutable: bool,
        initializer: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        let (key, exclusive) = expected;
        if mutable {
            return Err(self.error(span, "ZRYNA-M7007", "lexical loan binding must be constant"));
        }
        charge(self.spent, 1)?;
        let expression = &self.original.body.expressions[initializer as usize];
        let value = match &expression.kind {
            Expr::Borrow { value, .. } if !exclusive => *value,
            Expr::BorrowMut { value, .. } if exclusive => *value,
            _ => {
                return Err(self.error(
                    expression.span,
                    "ZRYNA-M7006",
                    "loan initializer must have matching explicit borrow access",
                ));
            }
        };
        let region = *self.regions.last().ok_or(Failure::InternalFailure)?;
        let target = self.source_place_in(value, depth + 2, Some(region))?;
        self.available_target(&target, expression.span)?;
        let ty = self.loan_referent(key)?;
        if ty != target.ty {
            return Err(self.error(
                expression.span,
                "ZRYNA-M7006",
                "borrow referent differs from declaration",
            ));
        }
        if exclusive && !target.mutable {
            return Err(self.error(
                expression.span,
                "ZRYNA-M7007",
                "exclusive borrow requires a mutable referent",
            ));
        }
        let origin = match target.source {
            raw::PlaceOrLoan::Place(id) => raw::LoanOrigin::Place(id),
            raw::PlaceOrLoan::Loan(id) => raw::LoanOrigin::InternalReborrow(id),
        };
        let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
        let id = self.add_loan(ty, access, origin, region, expression.span, true)?;
        self.bind_loan(name, id, ty, exclusive)
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn replace_loan(
        &mut self,
        target: u32,
        rhs: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        let mark = self.output.loans.len();
        let b = self.source_place_in(target, depth + 1, Some(span))?;
        let raw::PlaceOrLoan::Loan(id) = b.source else {
            return Err(reject(
                "ordinary static replacement depends on partial-place source replay",
            ));
        };
        if self.output.loans[id as usize].access != raw::Access::Exclusive {
            return Err(self.error(
                self.original.body.expressions[target as usize].span,
                "ZRYNA-M7007",
                "shared referent cannot be replaced",
            ));
        }
        self.available_target(&b, span)?;
        let value = self.expression(rhs, depth + 1)?;
        if value.ty != raw::ValueType::Stored(b.ty) {
            return Err(self.error(
                self.original.body.expressions[rhs as usize].span,
                "ZRYNA-M7006",
                "replacement RHS differs from exact referent",
            ));
        }
        self.available_target(&b, span)?;
        self.consume(value, span)?;
        self.available_target(&b, span)?;
        self.instruction(None, raw::Operation::ReplaceLoan { loan: id, value: value.id }, span)?;
        self.end_created_loans(mark, span)
    }
}
