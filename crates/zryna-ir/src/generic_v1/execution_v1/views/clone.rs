use super::{
    CloneNodeKind, VerifiedCleanupPlan, VerifiedLoan, VerifiedOperation, VerifiedPlace,
    VerifiedValue, raw,
};
use crate::generic_v1::execution_v1::VerifiedExecution;

/// Exact readable source role; formal referents never become fabricated local places.
#[derive(Clone, Copy)]
pub enum VerifiedCloneSource<'p, 'a> {
    /// Complete same-seal place.
    Place(VerifiedPlace<'p, 'a>),
    /// Active exact referent loan.
    Loan(VerifiedLoan<'p, 'a>),
}
/// A complete Clone operation and its local destination frontier.
#[derive(Clone, Copy)]
pub struct VerifiedClone<'p, 'a> {
    pub(super) operation: VerifiedOperation<'p, 'a>,
}
/// One finite same-seal recipe node; cycles through indirection retain graph references.
#[derive(Clone, Copy)]
pub struct VerifiedCloneNode<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    ty: u32,
}

impl<'p, 'a> VerifiedClone<'p, 'a> {
    /// Exact source binding, preserved through local rollback.
    #[must_use]
    pub fn source(self) -> VerifiedCloneSource<'p, 'a> {
        let raw::Operation::Clone { source, .. } = self.operation.kind() else {
            unreachable!("sealed Clone view")
        };
        let function = self.operation.site().function;
        match source {
            raw::PlaceOrLoan::Place(id) => VerifiedCloneSource::Place(VerifiedPlace {
                owner: self.operation.owner,
                function,
                id: *id,
            }),
            raw::PlaceOrLoan::Loan(id) => VerifiedCloneSource::Loan(VerifiedLoan {
                owner: self.operation.owner,
                function,
                id: *id,
            }),
        }
    }
    /// Distinct exact result identity; a Copy result need not invent an owning place.
    ///
    /// # Panics
    /// Panics only if the internal sealed Clone result invariant is broken.
    #[must_use]
    pub fn destination(self) -> VerifiedValue<'p, 'a> {
        self.operation.result().expect("sealed Clone result")
    }
    /// Exact ordinary result value, published after complete preparation.
    #[must_use]
    pub fn result(self) -> VerifiedValue<'p, 'a> {
        self.destination()
    }
    /// Pending destination place for affine preparation, never a runtime progress claim.
    #[must_use]
    pub fn destination_place(self) -> Option<VerifiedPlace<'p, 'a>> {
        self.operation.owner.plan.operations[self.operation.index].destination.map(|id| {
            VerifiedPlace {
                owner: self.operation.owner,
                function: self.operation.site().function,
                id,
            }
        })
    }
    /// Same closed type identity of source, result and frontier.
    #[must_use]
    pub fn ty(self) -> u32 {
        let raw::ValueType::Stored(ty) = self.destination().ty() else {
            unreachable!("sealed stored Clone type")
        };
        ty
    }
    /// Finite root node; Vec recursion is not statically unfolded.
    #[must_use]
    pub fn frontier(self) -> VerifiedCloneNode<'p, 'a> {
        VerifiedCloneNode { owner: self.operation.owner, ty: self.ty() }
    }
    /// Exact pre-existing root/loan cleanup after local prefix rollback.
    #[must_use]
    pub fn outer_failure(self) -> Option<VerifiedCleanupPlan<'p, 'a>> {
        self.operation.failure()
    }
    /// Exact symbolic prefix-first cleanup; no supplied initialized count is admitted.
    #[must_use]
    pub fn prefix_failure(self) -> Option<VerifiedCleanupPlan<'p, 'a>> {
        self.operation.owner.plan.operations[self.operation.index].prefix.map(|id| {
            VerifiedCleanupPlan {
                owner: self.operation.owner,
                function: self.operation.site().function,
                id,
            }
        })
    }
}
impl<'p, 'a> VerifiedCloneNode<'p, 'a> {
    /// Exact same-universe stored type ID.
    #[must_use]
    pub const fn ty(self) -> u32 {
        self.ty
    }
    /// Checked action whose capability came from source, not layout.
    #[must_use]
    pub fn kind(self) -> CloneNodeKind {
        self.node().kind
    }
    /// Whether preparation may perform a controlled fallible resource action.
    #[must_use]
    pub fn may_fail(self) -> bool {
        self.node().may_fail
    }
    /// Declaration-ordered exact Struct children.
    ///
    /// # Panics
    /// Panics only if a sealed child or its verified ordinal bound is corrupted.
    #[must_use]
    pub fn fields(self) -> impl ExactSizeIterator<Item = (u32, VerifiedCloneNode<'p, 'a>)> {
        let children = if self.kind() == CloneNodeKind::Struct {
            self.node().children.as_slice()
        } else {
            &[]
        };
        children.iter().enumerate().map(move |(ordinal, child)| {
            (
                u32::try_from(ordinal).expect("sealed bounded member ordinal"),
                Self { owner: self.owner, ty: child.expect("sealed Struct child") },
            )
        })
    }
    /// Every fixed ordinal; only the runtime-active payload executes.
    ///
    /// # Panics
    /// Panics only if an internal sealed variant index exceeds its verified bound.
    #[must_use]
    pub fn variants(
        self,
    ) -> impl ExactSizeIterator<Item = (u32, Option<VerifiedCloneNode<'p, 'a>>)> {
        let children =
            if self.kind() == CloneNodeKind::Enum { self.node().children.as_slice() } else { &[] };
        children.iter().enumerate().map(move |(ordinal, child)| {
            (
                u32::try_from(ordinal).expect("sealed bounded member ordinal"),
                child.map(|ty| Self { owner: self.owner, ty }),
            )
        })
    }
    /// Exact array/Vec element graph reference, including a same-key cycle.
    #[must_use]
    pub fn element(self) -> Option<VerifiedCloneNode<'p, 'a>> {
        matches!(self.kind(), CloneNodeKind::FixedArray { .. } | CloneNodeKind::VecEach)
            .then(|| self.node().children.first().copied().flatten())
            .flatten()
            .map(|ty| Self { owner: self.owner, ty })
    }
    fn node(self) -> &'p crate::generic_v1::execution_v1::frontier::Node {
        self.owner.typed.frontier.nodes[self.ty as usize].as_ref().expect("sealed clone node")
    }
}
