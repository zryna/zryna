//! Lexical loans and RHS-first replacement retain real source regions and identities.
use super::{Binding, Body, Expr, Failure, UntrustedSpan, charge, raw, reject};
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
        let mark = self.output.loans.len();
        let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
        let target = self.source_place_mode(
            value,
            depth + 2,
            Some(region),
            (access, raw::RegionRole::Lexical),
        )?;
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
        if let raw::PlaceOrLoan::Loan(id) = target.source
            && id as usize >= mark
            && matches!(self.output.loans[id as usize].origin, raw::LoanOrigin::Indexed { .. })
        {
            return self.bind_loan(name, id, ty, exclusive);
        }
        let origin = match target.source {
            raw::PlaceOrLoan::Place(id) => raw::LoanOrigin::Place(id),
            raw::PlaceOrLoan::Loan(id) => raw::LoanOrigin::InternalReborrow(id),
        };
        let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
        let id = self.add_loan(ty, access, origin, region, expression.span, true)?;
        self.bind_loan(name, id, ty, exclusive)
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn replace_binding(
        &mut self,
        b: &Binding<'s>,
        rhs: u32,
        span: UntrustedSpan,
        depth: usize,
        mark: usize,
    ) -> Result<(), Failure> {
        let raw::PlaceOrLoan::Loan(id) = b.source else {
            return Err(reject(
                "ordinary static replacement depends on partial-place source replay",
            ));
        };
        if self.output.loans[id as usize].access != raw::Access::Exclusive {
            return Err(self.error(span, "ZRYNA-M7007", "shared referent cannot be replaced"));
        }
        self.available_target(b, span)?;
        let value = self.expression(rhs, depth + 1)?;
        if value.ty != raw::ValueType::Stored(b.ty) {
            return Err(self.error(
                self.original.body.expressions[rhs as usize].span,
                "ZRYNA-M7006",
                "replacement RHS differs from exact referent",
            ));
        }
        self.available_target(b, span)?;
        self.consume(value, span)?;
        self.available_target(b, span)?;
        self.instruction(None, raw::Operation::ReplaceLoan { loan: id, value: value.id }, span)?;
        self.end_created_loans(mark, span)
    }
}
