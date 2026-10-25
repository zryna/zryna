//! Exact maximal initialized frontier, reverse declaration/index within reverse root order.
use super::{Body, Failure, charge, raw, reserve};
impl Body<'_, '_> {
    fn tree_stack(&mut self, id: u32) -> Result<Vec<u32>, Failure> {
        charge(
            self.spent,
            self.output
                .places
                .len()
                .checked_mul(size_of::<u32>() + 1)
                .ok_or(Failure::InternalFailure)?,
        )?;
        let mut stack = reserve(self.output.places.len())?;
        stack.push(id);
        Ok(stack)
    }
    pub(super) fn has_initialized_storage(&mut self, id: u32) -> Result<bool, Failure> {
        let mut stack = self.tree_stack(id)?;
        while let Some(at) = stack.pop() {
            charge(self.spent, 1)?;
            if self.state.live[at as usize] {
                return Ok(true);
            }
            if self.state.partial[at as usize] {
                charge(self.spent, self.children[at as usize].len())?;
                stack.extend_from_slice(&self.children[at as usize]);
            }
        }
        Ok(false)
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn partial_actions(
        &mut self,
    ) -> Result<Vec<raw::CleanupAction>, Failure> {
        let capacity = self
            .output
            .places
            .len()
            .checked_add(self.state.loan_order.len())
            .ok_or(Failure::InternalFailure)?;
        charge(
            self.spent,
            capacity
                .checked_mul(size_of::<raw::CleanupAction>() + 1)
                .ok_or(Failure::InternalFailure)?,
        )?;
        let mut actions = reserve(capacity)?;
        actions
            .extend(self.state.loan_order.iter().rev().copied().map(raw::CleanupAction::EndLoan));
        charge(self.spent, self.state.order.len())?;
        for index in (0..self.state.order.len()).rev() {
            let mut stack = self.tree_stack(self.state.order[index])?;
            while let Some(at) = stack.pop() {
                charge(self.spent, 1)?;
                if self.state.live[at as usize] {
                    if !self.copy(self.output.places[at as usize].ty)? {
                        actions.push(raw::CleanupAction::DropPlace(at));
                    }
                } else if self.state.partial[at as usize] {
                    charge(self.spent, self.children[at as usize].len())?;
                    stack.extend_from_slice(&self.children[at as usize]);
                }
            }
        }
        Ok(actions)
    }
}
