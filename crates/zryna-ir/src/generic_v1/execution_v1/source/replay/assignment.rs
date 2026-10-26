//! Ordinary assignment resolves storage before RHS and transfers the actual RHS pending anchor.
use super::{Binding, Body, Expr, Failure, UntrustedSpan, charge, raw};
impl<'s> Body<'_, 's> {
    pub(super) fn assign_source(
        &mut self,
        target: u32,
        rhs: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        let mark = self.output.loans.len();
        let binding = self.assignment_binding(target, span, depth)?;
        self.assignment_loan_roles(mark)?;
        let raw::PlaceOrLoan::Place(place) = binding.source else {
            return self.replace_binding(&binding, rhs, span, depth, mark);
        };
        if !binding.mutable {
            return Err(self.error(
                self.original.body.expressions[target as usize].span,
                "ZRYNA-M7006",
                "assignment requires a mutable source binding",
            ));
        }
        let value = self.expression(rhs, depth + 1)?;
        if value.ty != raw::ValueType::Stored(binding.ty) {
            return Err(self.error(
                self.original.body.expressions[rhs as usize].span,
                "ZRYNA-M7006",
                "assignment RHS differs from exact destination type",
            ));
        }
        self.install_source(place, value, span)
    }
    pub(super) fn assignment_loan_roles(&mut self, first: usize) -> Result<(), Failure> {
        charge(self.spent, self.output.loans.len() - first)?;
        let indexed = self.output.loans[first..].iter().any(|loan| {
            loan.region.role == raw::RegionRole::TransientIndexedAccess
                && matches!(loan.origin, raw::LoanOrigin::Indexed { .. })
        });
        if !indexed {
            charge(self.spent, self.output.loans.len() - first)?;
            for loan in &mut self.output.loans[first..] {
                if loan.region.role == raw::RegionRole::TransientIndexedAccess {
                    loan.region.role = raw::RegionRole::Lexical;
                }
            }
        }
        Ok(())
    }
    fn assignment_binding(
        &mut self,
        target: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Binding<'s>, Failure> {
        if !u32::try_from(depth).is_ok_and(|d| d <= zryna_syntax::v4::MAX_NESTING_DEPTH) {
            return Err(super::exhausted("assignment exceeds inherited syntax depth"));
        }
        charge(self.spent, 1)?;
        if let Expr::Reference { name } = &self.original.body.expressions[target as usize].kind {
            for binding in self.state.locals.iter().rev() {
                charge(self.spent, binding.name.len().min(name.text.len()) + 1)?;
                if binding.name == name.text {
                    return Ok(binding.clone());
                }
            }
        }
        self.source_place_mode(
            target,
            depth + 1,
            Some(span),
            (raw::Access::Exclusive, raw::RegionRole::TransientIndexedAccess),
        )
    }
}
