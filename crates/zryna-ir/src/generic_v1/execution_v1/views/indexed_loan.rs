//! Exact indexed origin metadata; liveness and runtime bounds belong to its operation.
use super::{VerifiedLoan, VerifiedOperation, VerifiedPlace, VerifiedValue, raw};
/// Checked indexed origin metadata, without runtime bounds-success or liveness authority.
#[derive(Clone, Copy)]
pub struct VerifiedIndexedLoan<'p, 'a> {
    loan: VerifiedLoan<'p, 'a>,
    base: raw::PlaceOrLoan,
    index: u32,
    container: u32,
}
impl<'p, 'a> VerifiedIndexedLoan<'p, 'a> {
    /// Exact metadata row retaining this origin.
    #[must_use]
    pub const fn loan(self) -> VerifiedLoan<'p, 'a> {
        self.loan
    }
    /// Exact prepared i32 index; dominance is checked at `BeginLoan`.
    #[must_use]
    pub fn index(self) -> VerifiedValue<'p, 'a> {
        VerifiedValue { owner: self.loan.owner, function: self.loan.function, id: self.index }
    }
    /// Exact local container base, absent for a loan-backed container.
    #[must_use]
    pub fn base_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        let raw::PlaceOrLoan::Place(id) = self.base else { return None };
        Some(VerifiedPlace { owner: self.loan.owner, function: self.loan.function, id })
    }
    /// Exact parent container loan, without fabricating a local Place.
    #[must_use]
    pub fn base_loan(self) -> Option<VerifiedLoan<'p, 'a>> {
        let raw::PlaceOrLoan::Loan(id) = self.base else { return None };
        Some(VerifiedLoan { id, ..self.loan })
    }
    /// Exact source-bound container type.
    #[must_use]
    pub const fn container_type(self) -> u32 {
        self.container
    }
}
impl<'p, 'a> VerifiedLoan<'p, 'a> {
    /// Checked indexed origin metadata only; not runtime bounds or live-loan authority.
    #[must_use]
    pub fn indexed(self) -> Option<VerifiedIndexedLoan<'p, 'a>> {
        let raw::LoanOrigin::Indexed { base, index } =
            self.owner.program.functions[self.function].loans[self.id as usize].origin
        else {
            return None;
        };
        let container = match base {
            raw::PlaceOrLoan::Place(id) => {
                self.owner.typed.functions[self.function].places.rows[id as usize].ty
            }
            raw::PlaceOrLoan::Loan(id) => {
                self.owner.program.functions[self.function].loans[id as usize].referent
            }
        };
        Some(VerifiedIndexedLoan { loan: self, base, index, container })
    }
}
impl<'p, 'a> VerifiedOperation<'p, 'a> {
    /// Exact checked indexed Begin operation; failure excludes the not-yet-live child loan.
    #[must_use]
    pub fn indexed_loan(self) -> Option<VerifiedIndexedLoan<'p, 'a>> {
        let raw::Operation::BeginLoan { loan, .. } = self.kind() else { return None };
        VerifiedLoan { owner: self.owner, function: self.site().function, id: *loan }.indexed()
    }
}
