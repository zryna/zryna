//! Exact append preparation; result ownership is never synthesized.
use super::{
    VerifiedCleanupPlan, VerifiedLoan, VerifiedOperation, VerifiedPlace, VerifiedValue, raw,
};
/// Checked Vec mutation and prepared RHS from one operation's retained execution seal.
#[derive(Clone, Copy)]
pub struct VerifiedVecPush<'p, 'a> {
    operation: VerifiedOperation<'p, 'a>,
}
impl<'p, 'a> VerifiedOperation<'p, 'a> {
    /// Exact checked append with target rechecked after RHS preparation.
    #[must_use]
    pub fn vec_push(self) -> Option<VerifiedVecPush<'p, 'a>> {
        matches!(self.kind(), raw::Operation::VecPush { .. })
            .then_some(VerifiedVecPush { operation: self })
    }
}
impl<'p, 'a> VerifiedVecPush<'p, 'a> {
    /// Exact initialized mutable static place, absent for an exclusive loan target.
    #[must_use]
    pub fn target_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        super::containers::source_place(self.operation, self.target())
    }
    /// Exact live exclusive loan, absent for a direct mutable place target.
    #[must_use]
    pub fn target_loan(self) -> Option<VerifiedLoan<'p, 'a>> {
        super::containers::source_loan(self.operation, self.target())
    }
    /// Complete original Vec identity.
    #[must_use]
    pub fn ty(self) -> u32 {
        let function = self.operation.site().function;
        match self.target() {
            raw::PlaceOrLoan::Place(id) => {
                self.operation.owner.typed.functions[function].places.rows[id as usize].ty
            }
            raw::PlaceOrLoan::Loan(id) => {
                self.operation.owner.program.functions[function].loans[id as usize].referent
            }
        }
    }
    /// Exact original element identity, independent of layout capability flags.
    #[must_use]
    pub fn element_type(self) -> u32 {
        self.child()
    }
    fn child(self) -> u32 {
        self.operation.owner.source.world().arguments(self.ty()).expect("sealed Vec child")[0]
    }
    /// Already prepared exact element; consumed only by successful append.
    #[must_use]
    pub fn value(self) -> VerifiedValue<'p, 'a> {
        let raw::Operation::VecPush { value, .. } = self.operation.kind() else {
            unreachable!("sealed append")
        };
        super::containers::value(self.operation, *value)
    }
    /// Growth failure retains both original vector and prepared RHS owners.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        super::containers::failure(self.operation)
    }
    fn target(self) -> raw::PlaceOrLoan {
        let raw::Operation::VecPush { vector, .. } = self.operation.kind() else {
            unreachable!("sealed append")
        };
        *vector
    }
}
