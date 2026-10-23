//! Exact reverse maximal initialized subobjects; no empty String lane is inspected.
use super::{Context, Failure, Init, State, raw, reject};
use crate::generic_v1::execution_v1::{budget, reserve, resource_failure};
impl State {
    pub(in crate::generic_v1::execution_v1::state) fn drop_targets(
        &self,
        c: &Context<'_>,
        id: u32,
        work: &mut usize,
    ) -> Result<(Vec<raw::PlaceOrLoan>, bool), Failure> {
        self.active(c, id)?;
        self.loan_free(c, id, true)?;
        let kind = self.status(c, id).0;
        if kind == Init::Empty {
            return Err(reject("drop or replacement targets uninitialized storage"));
        }
        if kind == Init::Whole {
            budget::charge(work, 2)?;
            let mut result = reserve(1)?;
            result.push(raw::PlaceOrLoan::Place(id));
            return Ok((result, true));
        }
        self.frontier_credit(c, work)?;
        let mut stack = reserve(c.raw().places.len())?;
        let mut result = reserve(c.raw().places.len())?;
        self.walk_drop(c, id, &mut stack, &mut result)?;
        Ok((result, false))
    }
    fn frontier_credit(&self, c: &Context<'_>, work: &mut usize) -> Result<(), Failure> {
        budget::charge(
            work,
            c.raw()
                .places
                .len()
                .checked_mul(8)
                .and_then(|n| n.checked_add(self.loan_order.len()))
                .ok_or_else(|| resource_failure("partial cleanup frontier overflow"))?,
        )
    }
    fn walk_drop(
        &self,
        c: &Context<'_>,
        id: u32,
        stack: &mut Vec<u32>,
        result: &mut Vec<raw::PlaceOrLoan>,
    ) -> Result<(), Failure> {
        let d = &c.checked().places.decomposition;
        stack.push(d.canonical(id));
        while let Some(at) = stack.pop() {
            if self.initialized[at as usize] {
                if c.affine(raw::ValueType::Stored(c.checked().places.rows[at as usize].ty))? {
                    result.push(raw::PlaceOrLoan::Place(at));
                }
            } else if self.partial[at as usize] {
                stack.extend(d.complete(at, self.variants[at as usize])?);
            }
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::state) fn actions(
        &self,
        c: &Context<'_>,
        work: &mut usize,
    ) -> Result<Vec<raw::CleanupAction>, Failure> {
        budget::charge(work, self.partial.len())?;
        if !self.partial.iter().any(|p| *p) {
            return self.whole_actions(work);
        }
        self.frontier_credit(c, work)?;
        let capacity = c
            .raw()
            .places
            .len()
            .checked_add(self.loan_order.len())
            .ok_or_else(|| resource_failure("partial cleanup action capacity overflow"))?;
        let mut actions = reserve(capacity)?;
        let mut stack = reserve(c.raw().places.len())?;
        let mut targets = reserve(c.raw().places.len())?;
        actions.extend(self.loan_order.iter().rev().copied().map(raw::CleanupAction::EndLoan));
        for &owner in self.owners.iter().rev() {
            targets.clear();
            self.walk_drop(c, owner, &mut stack, &mut targets)?;
            actions.extend(targets.iter().map(|target| match target {
                raw::PlaceOrLoan::Place(id) => raw::CleanupAction::DropPlace(*id),
                raw::PlaceOrLoan::Loan(_) => unreachable!("owned root frontier contains places"),
            }));
        }
        Ok(actions)
    }
}
