//! Exact lexical snapshots and reverse cleanup; ordinary values and loans stay distinct.
use super::{Binding, Body, Failure, State, UntrustedSpan, charge, raw, reject, reserve};
impl<'s> Body<'_, 's> {
    pub(super) fn bind(&mut self, name: &'s str, place: u32, ty: u32) -> Result<(), Failure> {
        self.bind_mutable(name, place, ty, false)
    }
    pub(super) fn bind_mutable(
        &mut self,
        name: &'s str,
        place: u32,
        ty: u32,
        mutable: bool,
    ) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        self.state.locals.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.locals.push(Binding {
            name,
            source: raw::PlaceOrLoan::Place(place),
            ty,
            mutable,
        });
        Ok(())
    }
    pub(super) fn named(
        &mut self,
        name: &str,
        span: UntrustedSpan,
    ) -> Result<Binding<'s>, Failure> {
        for index in (0..self.state.locals.len()).rev() {
            let b = &self.state.locals[index];
            charge(self.spent, b.name.len().min(name.len()) + 1)?;
            if b.name == name {
                if let raw::PlaceOrLoan::Loan(id) = b.source
                    && !self.state.loans[id as usize]
                {
                    return Err(self.error(span, "ZRYNA-M7007", "source loan is no longer live"));
                }
                return Ok(b.clone());
            }
        }
        Err(reject("source references an unknown lexical binding"))
    }
    pub(super) fn actions(&mut self) -> Result<Vec<raw::CleanupAction>, Failure> {
        charge(self.spent, self.state.partial.len())?;
        if self.state.partial.iter().any(|partial| *partial) {
            return self.partial_actions();
        }
        charge(self.spent, self.state.order.len() + self.state.loan_order.len())?;
        let mut a = reserve(self.state.order.len() + self.state.loan_order.len())?;
        a.extend(self.state.loan_order.iter().rev().copied().map(raw::CleanupAction::EndLoan));
        a.extend(self.state.order.iter().rev().copied().map(raw::CleanupAction::DropPlace));
        Ok(a)
    }
    pub(super) fn snapshot(&mut self) -> Result<State<'s>, Failure> {
        charge(
            self.spent,
            self.state.live.len()
                + self.state.partial.len() * 2
                + self.state.order.len()
                + self.state.locals.len()
                + self.state.loans.len()
                + self.state.loan_order.len(),
        )?;
        let mut live = reserve(self.state.live.len())?;
        live.extend_from_slice(&self.state.live);
        let mut partial = reserve(self.state.partial.len())?;
        partial.extend_from_slice(&self.state.partial);
        let mut order = reserve(self.state.order.len())?;
        order.extend_from_slice(&self.state.order);
        let mut locals = reserve(self.state.locals.len())?;
        locals.extend(self.state.locals.iter().cloned());
        let mut loans = reserve(self.state.loans.len())?;
        loans.extend_from_slice(&self.state.loans);
        let mut loan_order = reserve(self.state.loan_order.len())?;
        loan_order.extend_from_slice(&self.state.loan_order);
        Ok(State { locals, live, partial, order, loans, loan_order })
    }
    pub(super) fn restore(&mut self, mut state: State<'s>) -> Result<(), Failure> {
        charge(self.spent, self.output.places.len())?;
        state
            .live
            .try_reserve(self.output.places.len() - state.live.len())
            .map_err(|_| Failure::AllocationFailure)?;
        state.live.resize(self.output.places.len(), false);
        charge(
            self.spent,
            self.output.places.len().checked_mul(2).ok_or(Failure::InternalFailure)?,
        )?;
        state
            .partial
            .try_reserve(self.output.places.len() - state.partial.len())
            .map_err(|_| Failure::AllocationFailure)?;
        state.partial.resize(self.output.places.len(), false);
        charge(self.spent, self.output.loans.len())?;
        state
            .loans
            .try_reserve(self.output.loans.len() - state.loans.len())
            .map_err(|_| Failure::AllocationFailure)?;
        state.loans.resize(self.output.loans.len(), false);
        self.state = state;
        Ok(())
    }
}
