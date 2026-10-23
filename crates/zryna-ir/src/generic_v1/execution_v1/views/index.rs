//! Exact Copy observation of a retained Vec or fixed array.
use super::containers::{failure, result, source_loan, source_place, stored, value};
use super::{
    VerifiedCleanupPlan, VerifiedLoan, VerifiedOperation, VerifiedPlace, VerifiedValue, raw,
};
/// One independently checked index, with no affine dynamic-place fabrication.
#[derive(Clone, Copy)]
pub struct VerifiedIndexCopy<'p, 'a> {
    pub(super) operation: VerifiedOperation<'p, 'a>,
}
impl<'p, 'a> VerifiedIndexCopy<'p, 'a> {
    /// Original Copy result, available only after successful bounds observation.
    #[must_use]
    pub fn result(self) -> VerifiedValue<'p, 'a> {
        result(self.operation)
    }
    /// Exact original Copy element identity.
    #[must_use]
    pub fn element_type(self) -> u32 {
        stored(self.operation)
    }
    /// Exact source Vec/array identity; no equal-sized Struct can substitute.
    #[must_use]
    pub fn container_type(self) -> u32 {
        let function = self.operation.site().function;
        match self.source() {
            raw::PlaceOrLoan::Place(id) => {
                self.operation.owner.typed.functions[function].places.rows[id as usize].ty
            }
            raw::PlaceOrLoan::Loan(id) => {
                self.operation.owner.program.functions[function].loans[id as usize].referent
            }
        }
    }
    /// Exact i32 index value, already prepared before bounds observation.
    #[must_use]
    pub fn index(self) -> VerifiedValue<'p, 'a> {
        let raw::Operation::IndexCopy { index, .. } = self.operation.kind() else {
            unreachable!("sealed Copy index")
        };
        value(self.operation, *index)
    }
    /// Exact readable container place, absent for a loan source.
    #[must_use]
    pub fn source_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        source_place(self.operation, self.source())
    }
    /// Exact readable container loan, absent for a place source.
    #[must_use]
    pub fn source_loan(self) -> Option<VerifiedLoan<'p, 'a>> {
        source_loan(self.operation, self.source())
    }
    /// Bounds failure retains the entire source container.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        failure(self.operation)
    }
    fn source(self) -> raw::PlaceOrLoan {
        let raw::Operation::IndexCopy { base, .. } = self.operation.kind() else {
            unreachable!("sealed Copy index")
        };
        *base
    }
}
