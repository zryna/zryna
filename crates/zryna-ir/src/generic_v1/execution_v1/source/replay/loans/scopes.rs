//! Loan liveness snapshots and exact source-contained endings precede owned cleanup.
use super::{Body, Failure, UntrustedSpan, charge, raw};
impl Body<'_, '_> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn require_live_loan(
        &mut self,
        id: u32,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        if !self.state.loans.get(id as usize).copied().unwrap_or(false) {
            return Err(self.error(span, "ZRYNA-M7007", "source loan is no longer live"));
        }
        Ok(())
    }
    fn end_source_loan(&mut self, id: u32, site: UntrustedSpan) -> Result<(), Failure> {
        self.require_live_loan(id, site)?;
        charge(self.spent, self.state.loan_order.len())?;
        let position = self
            .state
            .loan_order
            .iter()
            .position(|live| *live == id)
            .ok_or(Failure::InternalFailure)?;
        self.instruction(None, raw::Operation::EndLoan { loan: id }, site)?;
        self.state.loans[id as usize] = false;
        self.state.loan_order.remove(position);
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn end_created_loans(
        &mut self,
        first: usize,
        site: UntrustedSpan,
    ) -> Result<(), Failure> {
        charge(self.spent, self.state.loan_order.len())?;
        let mut at = self.state.loan_order.len();
        while at > 0 {
            at -= 1;
            let id = self.state.loan_order[at];
            if id as usize >= first {
                self.end_source_loan(id, site)?;
            }
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn end_scope_loans(
        &mut self,
        locals: usize,
    ) -> Result<(), Failure> {
        charge(self.spent, self.state.loan_order.len())?;
        let mut at = self.state.loan_order.len();
        while at > 0 {
            at -= 1;
            let id = self.state.loan_order[at];
            if self.loan_scopes[id as usize] >= locals {
                let site = self.output.loans[id as usize].region.span;
                self.end_source_loan(id, site)?;
            }
        }
        Ok(())
    }
}
