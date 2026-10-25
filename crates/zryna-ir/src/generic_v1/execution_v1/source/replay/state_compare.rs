//! Canonical source flags agree exactly; later-created inactive rows are zero padded.
use super::{Body, Failure, State, charge};
impl Body<'_, '_> {
    pub(super) fn compare_source_state(&mut self, other: &State<'_>) -> Result<(), Failure> {
        let count = self.state.live.len().max(other.live.len());
        let loans = self.state.loans.len().max(other.loans.len());
        let cost = count
            .checked_mul(4)
            .and_then(|n| n.checked_add(loans.checked_mul(2)?))
            .and_then(|n| n.checked_add(self.state.order.len()))
            .and_then(|n| n.checked_add(other.order.len()))
            .and_then(|n| n.checked_add(self.state.loan_order.len()))
            .and_then(|n| n.checked_add(other.loan_order.len()))
            .ok_or(Failure::InternalFailure)?;
        charge(self.spent, cost)?;
        let flags = (0..count).all(|id| {
            self.state.live.get(id).copied().unwrap_or(false)
                == other.live.get(id).copied().unwrap_or(false)
                && self.state.partial.get(id).copied().unwrap_or(false)
                    == other.partial.get(id).copied().unwrap_or(false)
        }) && (0..loans).all(|id| {
            self.state.loans.get(id).copied().unwrap_or(false)
                == other.loans.get(id).copied().unwrap_or(false)
        });
        if !flags || self.state.order != other.order || self.state.loan_order != other.loan_order {
            return Err(self.error(
                self.original.span,
                "ZRYNA-M7007",
                "conditional source paths have different initialized subtrees or pending owners",
            ));
        }
        Ok(())
    }
}
