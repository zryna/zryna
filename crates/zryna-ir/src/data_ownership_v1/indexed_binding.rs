//! Infallible finalization of a transient indexed access into a lexical authority.

use super::{
    BorrowIdentity, LayoutTypeId, PlaceIdentity, VerifiedBorrowAccess, VerifiedInstruction, raw,
};

/// Exact lexical authority transferred from one completed transient access.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedIndexedBinding) {
///     view.parent = view.borrow();
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedIndexedBinding {
    parent: BorrowIdentity,
    borrow: BorrowIdentity,
    container: PlaceIdentity,
    referent: LayoutTypeId,
    access: VerifiedBorrowAccess,
}

#[allow(missing_docs)]
impl VerifiedIndexedBinding {
    #[must_use]
    pub const fn parent(self) -> BorrowIdentity {
        self.parent
    }
    #[must_use]
    pub const fn borrow(self) -> BorrowIdentity {
        self.borrow
    }
    #[must_use]
    pub const fn container(self) -> PlaceIdentity {
        self.container
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> VerifiedBorrowAccess {
        self.access
    }
}

impl VerifiedInstruction<'_> {
    /// Finalizes a transient access without bounds, allocation, or owner transfer.
    #[must_use]
    pub fn indexed_binding(self) -> Option<VerifiedIndexedBinding> {
        let raw::InstructionKind::BindIndexedBorrow { parent, borrow } = self.instruction.kind
        else {
            return None;
        };
        let borrows = self.function.borrows();
        let (ty, access) = borrows.definition(borrow)?;
        let region = borrows.region(borrow)?;
        let owner = self.function.id();
        Some(VerifiedIndexedBinding {
            parent: BorrowIdentity { owner, index: parent.0 },
            borrow: BorrowIdentity { owner, index: borrow.0 },
            container: PlaceIdentity { owner, index: region.0 },
            referent: super::layout_type(&self.function.owner.linear32, ty)?.id(),
            access: access.into(),
        })
    }
}
