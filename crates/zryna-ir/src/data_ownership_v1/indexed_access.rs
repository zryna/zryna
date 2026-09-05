//! Transient indexing shares lexical bounds authority without consuming lexical aliases.

use super::{BorrowIndex, Errors, error_at, layout_type, raw};

/// A checked child access retaining its original container conflict region.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedIndexedProjection) {
///     view.array_length = Some(0);
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedIndexedProjection {
    parent: super::BorrowIdentity,
    borrow: super::BorrowIdentity,
    container: super::PlaceIdentity,
    index: super::ValueIdentity,
    referent: super::LayoutTypeId,
    access: super::VerifiedBorrowAccess,
    cleanup: super::CleanupPlanIdentity,
    array_length: Option<u64>,
}

#[allow(missing_docs)]
impl VerifiedIndexedProjection {
    #[must_use]
    pub const fn parent(self) -> super::BorrowIdentity {
        self.parent
    }
    #[must_use]
    pub const fn borrow(self) -> super::BorrowIdentity {
        self.borrow
    }
    #[must_use]
    pub const fn container(self) -> super::PlaceIdentity {
        self.container
    }
    #[must_use]
    pub const fn index(self) -> super::ValueIdentity {
        self.index
    }
    #[must_use]
    pub const fn referent(self) -> super::LayoutTypeId {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> super::VerifiedBorrowAccess {
        self.access
    }
    #[must_use]
    pub const fn cleanup(self) -> super::CleanupPlanIdentity {
        self.cleanup
    }
    #[must_use]
    /// Fixed length, or `None` when the parent referent supplies a runtime Vec length.
    pub const fn array_length(self) -> Option<u64> {
        self.array_length
    }
    #[must_use]
    pub const fn trap_identity(self) -> super::VerifiedTrapIdentity {
        super::VerifiedTrapIdentity::BoundsV1
    }
}

impl super::VerifiedInstruction<'_> {
    /// Failure retains the parent; success atomically replaces it with the child.
    #[must_use]
    pub fn indexed_projection(self) -> Option<VerifiedIndexedProjection> {
        let raw::InstructionKind::ProjectIndexedBorrow { parent, borrow, index, cleanup } =
            self.instruction.kind
        else {
            return None;
        };
        let borrows = self.function.borrows();
        let layouts = &self.function.owner.linear32;
        let (source, _) = borrows.origin(parent)?;
        let (ty, access) = borrows.definition(parent)?;
        let record = layout_type(layouts, ty)?;
        let owner = self.function.id();
        Some(VerifiedIndexedProjection {
            parent: super::BorrowIdentity { owner, index: parent.0 },
            borrow: super::BorrowIdentity { owner, index: borrow.0 },
            container: super::PlaceIdentity { owner, index: source.place.0 },
            index: super::ValueIdentity { owner, index: index.0 },
            referent: record.referenced_type()?,
            access: access.into(),
            cleanup: super::CleanupPlanIdentity { owner, index: cleanup.0 },
            array_length: record.array_length(),
        })
    }
}

pub(super) fn apply_projection(
    borrows: &BorrowIndex,
    parent: raw::BorrowId,
    borrow: raw::BorrowId,
    span: zryna_source::Span,
    active: &mut Vec<Option<(raw::PlaceId, raw::BorrowAccess)>>,
    errors: &mut Errors,
) {
    let authority = active.get(parent.0 as usize).copied().flatten();
    if borrows.origin(parent).is_none()
        || parent.0 >= borrow.0
        || authority.is_none()
        || active.get(borrow.0 as usize).is_some_and(Option::is_some)
    {
        errors.push(error_at("ZRYNA-I3011", span,
            "indexed projection requires one active transient parent authority",
            "project a fresh child from a checked temporary access, never a lexical or formal alias"));
        return;
    }
    active[parent.0 as usize] = None;
    active.resize(active.len().max(borrow.0 as usize + 1), None);
    active[borrow.0 as usize] = authority;
}
