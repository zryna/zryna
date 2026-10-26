//! Getter-only preparation views, branded by one source/type/state/plan-verified operation.
use super::{
    VerifiedCleanupPlan, VerifiedLoan, VerifiedOperation, VerifiedPlace, VerifiedValue, raw,
};
/// Exact Vec allocation followed by source-ordered prepared element commitment.
#[derive(Clone, Copy)]
pub struct VerifiedVecConstruct<'p, 'a> {
    pub(super) operation: VerifiedOperation<'p, 'a>,
}
/// Exact control allocation followed by commitment of its prepared payload.
#[derive(Clone, Copy)]
pub struct VerifiedSharedConstruct<'p, 'a> {
    pub(super) operation: VerifiedOperation<'p, 'a>,
}
/// Exact checked weak-count acquisition; source Shared remains initialized.
#[derive(Clone, Copy)]
pub struct VerifiedWeakDowngrade<'p, 'a> {
    pub(super) operation: VerifiedOperation<'p, 'a>,
}
impl<'p, 'a> VerifiedVecConstruct<'p, 'a> {
    /// Fresh same-function result, available only after successful preparation.
    #[must_use]
    pub fn result(self) -> VerifiedValue<'p, 'a> {
        result(self.operation)
    }
    /// Exact affine destination; this is never a runtime initialized-count claim.
    #[must_use]
    pub fn destination_place(self) -> VerifiedPlace<'p, 'a> {
        destination(self.operation)
    }
    /// Complete source-built Vec type identity.
    #[must_use]
    pub fn ty(self) -> u32 {
        stored(self.operation)
    }
    /// Complete element identity shared by every prepared operand.
    #[must_use]
    pub fn element_type(self) -> u32 {
        child(self.operation)
    }
    /// Source evaluation order; every pending affine element is in allocation failure cleanup.
    #[must_use]
    pub fn elements(self) -> impl ExactSizeIterator<Item = VerifiedValue<'p, 'a>> {
        let raw::Operation::VecConstruct { elements, .. } = self.operation.kind() else {
            unreachable!("sealed Vec construction")
        };
        elements.iter().map(move |id| value(self.operation, *id))
    }
    /// Exact failure before any element commitment or destination publication.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        failure(self.operation)
    }
}
impl<'p, 'a> VerifiedSharedConstruct<'p, 'a> {
    /// Fresh control-owner result, available only after successful preparation.
    #[must_use]
    pub fn result(self) -> VerifiedValue<'p, 'a> {
        result(self.operation)
    }
    /// Exact unpublished affine result destination.
    #[must_use]
    pub fn destination_place(self) -> VerifiedPlace<'p, 'a> {
        destination(self.operation)
    }
    /// Complete source-built Shared type identity.
    #[must_use]
    pub fn ty(self) -> u32 {
        stored(self.operation)
    }
    /// Exact payload identity; construction imposes no payload Clone requirement.
    #[must_use]
    pub fn referent_type(self) -> u32 {
        child(self.operation)
    }
    /// Prepared payload, still owned by outer failure cleanup until control allocation succeeds.
    #[must_use]
    pub fn payload(self) -> VerifiedValue<'p, 'a> {
        let raw::Operation::SharedConstruct { value: id, .. } = self.operation.kind() else {
            unreachable!("sealed Shared construction")
        };
        value(self.operation, *id)
    }
    /// Exact outer cleanup excludes the unwritten Shared result.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        failure(self.operation)
    }
}
impl<'p, 'a> VerifiedWeakDowngrade<'p, 'a> {
    /// Fresh Weak owner after successful checked count acquisition.
    #[must_use]
    pub fn result(self) -> VerifiedValue<'p, 'a> {
        result(self.operation)
    }
    /// Exact unpublished affine result destination.
    #[must_use]
    pub fn destination_place(self) -> VerifiedPlace<'p, 'a> {
        destination(self.operation)
    }
    /// Complete Weak type identity; the source is exact Shared of the same referent.
    #[must_use]
    pub fn ty(self) -> u32 {
        stored(self.operation)
    }
    /// Exact unchanged referent identity; downgrade never clones or dereferences it.
    #[must_use]
    pub fn referent_type(self) -> u32 {
        child(self.operation)
    }
    /// Readable exact source place, absent when the source is a loan.
    #[must_use]
    pub fn source_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        source_place(self.operation, self.source())
    }
    /// Readable exact source loan, absent when the source is a place.
    #[must_use]
    pub fn source_loan(self) -> Option<VerifiedLoan<'p, 'a>> {
        source_loan(self.operation, self.source())
    }
    /// Count failure retains the source and excludes the unwritten Weak result.
    #[must_use]
    pub fn outer_failure(self) -> VerifiedCleanupPlan<'p, 'a> {
        failure(self.operation)
    }
    fn source(self) -> raw::PlaceOrLoan {
        let raw::Operation::WeakDowngrade { shared, .. } = self.operation.kind() else {
            unreachable!("sealed downgrade")
        };
        *shared
    }
}
pub(super) fn value<'p, 'a>(
    operation: VerifiedOperation<'p, 'a>,
    id: u32,
) -> VerifiedValue<'p, 'a> {
    VerifiedValue { owner: operation.owner, function: operation.site().function, id }
}
pub(super) fn result<'p, 'a>(operation: VerifiedOperation<'p, 'a>) -> VerifiedValue<'p, 'a> {
    operation.result().expect("sealed stored preparation result")
}
pub(super) fn stored(operation: VerifiedOperation<'_, '_>) -> u32 {
    let raw::ValueType::Stored(ty) = result(operation).ty() else {
        unreachable!("sealed stored preparation result")
    };
    ty
}
fn destination<'p, 'a>(operation: VerifiedOperation<'p, 'a>) -> VerifiedPlace<'p, 'a> {
    VerifiedPlace {
        owner: operation.owner,
        function: operation.site().function,
        id: operation.owner.plan.operations[operation.index]
            .destination
            .expect("sealed affine preparation destination"),
    }
}
fn child(operation: VerifiedOperation<'_, '_>) -> u32 {
    operation.owner.source.world().arguments(stored(operation)).expect("sealed container argument")
        [0]
}
pub(super) fn failure<'p, 'a>(operation: VerifiedOperation<'p, 'a>) -> VerifiedCleanupPlan<'p, 'a> {
    operation.failure().expect("sealed mandatory preparation failure")
}
pub(super) fn source_place<'p, 'a>(
    operation: VerifiedOperation<'p, 'a>,
    source: raw::PlaceOrLoan,
) -> Option<VerifiedPlace<'p, 'a>> {
    match source {
        raw::PlaceOrLoan::Place(id) => {
            Some(VerifiedPlace { owner: operation.owner, function: operation.site().function, id })
        }
        raw::PlaceOrLoan::Loan(_) => None,
    }
}
pub(super) fn source_loan<'p, 'a>(
    operation: VerifiedOperation<'p, 'a>,
    source: raw::PlaceOrLoan,
) -> Option<VerifiedLoan<'p, 'a>> {
    match source {
        raw::PlaceOrLoan::Loan(id) => {
            Some(VerifiedLoan { owner: operation.owner, function: operation.site().function, id })
        }
        raw::PlaceOrLoan::Place(_) => None,
    }
}
