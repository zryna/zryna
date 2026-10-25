//! Explicit inline call loans end with their actual source call.
use super::super::reject;
use super::{Binding, Body, Expr, Failure, UntrustedSpan, charge, raw};
impl<'s> Body<'_, 's> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn loan_argument(
        &mut self,
        at: u32,
        referent: u32,
        exclusive: bool,
        region: UntrustedSpan,
        depth: usize,
    ) -> Result<u32, Failure> {
        charge(self.spent, 1)?;
        let source = &self.original.body.expressions[at as usize];
        let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
        let binding = match &source.kind {
            Expr::Reference { .. } => self.reference(at)?,
            Expr::Borrow { value, .. } if !exclusive => {
                self.inline_loan_source(*value, referent, access, region, source.span, depth)?
            }
            Expr::BorrowMut { value, .. } if exclusive => {
                self.inline_loan_source(*value, referent, access, region, source.span, depth)?
            }
            _ => {
                return Err(self.error(
                    source.span,
                    "ZRYNA-M7006",
                    "loan call argument requires matching explicit source",
                ));
            }
        };
        let raw::PlaceOrLoan::Loan(id) = binding.source else {
            return Err(reject("borrow parameter requires exact source loan"));
        };
        if binding.ty != referent || self.output.loans[id as usize].access != access {
            return Err(self.error(
                source.span,
                "ZRYNA-M7006",
                "loan call argument differs from exact referent/access",
            ));
        }
        self.available_target(&binding, source.span)?;
        Ok(id)
    }
    fn inline_loan_source(
        &mut self,
        value: u32,
        referent: u32,
        access: raw::Access,
        region: UntrustedSpan,
        site: UntrustedSpan,
        depth: usize,
    ) -> Result<Binding<'s>, Failure> {
        let first = self.output.loans.len();
        let target = self.source_place_mode(
            value,
            depth + 1,
            Some(region),
            (access, raw::RegionRole::Lexical),
        )?;
        self.available_target(&target, site)?;
        if target.ty != referent || (access == raw::Access::Exclusive && !target.mutable) {
            return Err(self.error(
                site,
                "ZRYNA-M7006",
                "inline borrow differs from exact source type/access",
            ));
        }
        if let raw::PlaceOrLoan::Loan(id) = target.source
            && id as usize >= first
            && matches!(self.output.loans[id as usize].origin, raw::LoanOrigin::Indexed { .. })
        {
            return Ok(target);
        }
        let origin = match target.source {
            raw::PlaceOrLoan::Place(id) => raw::LoanOrigin::Place(id),
            raw::PlaceOrLoan::Loan(id) => raw::LoanOrigin::InternalReborrow(id),
        };
        self.indexed_loan_credit()?;
        let id = self.add_loan(referent, access, origin, region, site, true)?;
        Ok(Binding {
            name: "",
            source: raw::PlaceOrLoan::Loan(id),
            ty: referent,
            mutable: access == raw::Access::Exclusive,
        })
    }
}
