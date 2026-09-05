//! Sealed dynamic element borrowing and prepared referent replacement.

use super::{
    BorrowIdentity, CleanupPlanIdentity, Errors, LayoutTypeId, PlaceIdentity, TypeCategory,
    ValueIdentity, VerifiedBorrowAccess, VerifiedInstruction, VerifiedLayouts,
    VerifiedTrapIdentity, borrow_definition, consuming_instruction_operands, instruction_cleanup,
    is_projection_below, layout_type, lexical_borrow_place, overlaps_active, ownership_error, raw,
};

/// Exact checked element authority with a conservative whole-container conflict region.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedIndexedBorrow) {
///     view.array_length = Some(0);
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedIndexedBorrow {
    borrow: BorrowIdentity,
    container: PlaceIdentity,
    index: ValueIdentity,
    referent: LayoutTypeId,
    access: VerifiedBorrowAccess,
    cleanup: CleanupPlanIdentity,
    array_length: Option<u64>,
}

#[allow(missing_docs)]
impl VerifiedIndexedBorrow {
    #[must_use]
    pub const fn borrow(self) -> BorrowIdentity {
        self.borrow
    }
    #[must_use]
    pub const fn container(self) -> PlaceIdentity {
        self.container
    }
    #[must_use]
    pub const fn index(self) -> ValueIdentity {
        self.index
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.referent
    }
    #[must_use]
    pub const fn access(self) -> VerifiedBorrowAccess {
        self.access
    }
    #[must_use]
    pub const fn cleanup(self) -> CleanupPlanIdentity {
        self.cleanup
    }
    #[must_use]
    pub const fn array_length(self) -> Option<u64> {
        self.array_length
    }
    #[must_use]
    pub const fn trap_identity(self) -> VerifiedTrapIdentity {
        VerifiedTrapIdentity::BoundsV1
    }
}

/// Authority to drop the old fully initialized referent, never its containing owner.
/// Recursive cleanup follows runtime enum tags and vector lengths.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedBorrowReferentDrop,
///          borrow: zryna_ir::data_ownership_v1::BorrowIdentity) {
///     view.borrow = borrow;
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedBorrowReferentDrop {
    borrow: BorrowIdentity,
    referent: LayoutTypeId,
}

#[allow(missing_docs)]
impl VerifiedBorrowReferentDrop {
    #[must_use]
    pub const fn borrow(self) -> BorrowIdentity {
        self.borrow
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.referent
    }
}

/// Infallible commit of a prepared non-Copy value through an exclusive borrow.
///
/// ```compile_fail
/// fn forge(mut view: zryna_ir::data_ownership_v1::VerifiedBorrowReplacement,
///          value: zryna_ir::data_ownership_v1::ValueIdentity) {
///     view.value = value;
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedBorrowReplacement {
    old_value_drop: VerifiedBorrowReferentDrop,
    value: ValueIdentity,
}

#[allow(missing_docs)]
impl VerifiedBorrowReplacement {
    #[must_use]
    pub const fn borrow(self) -> BorrowIdentity {
        self.old_value_drop.borrow
    }
    #[must_use]
    pub const fn referent(self) -> LayoutTypeId {
        self.old_value_drop.referent
    }
    #[must_use]
    pub const fn value(self) -> ValueIdentity {
        self.value
    }
    #[must_use]
    pub const fn old_value_drop(self) -> VerifiedBorrowReferentDrop {
        self.old_value_drop
    }
}

impl VerifiedInstruction<'_> {
    /// Exact reverse-begin-order authorities discharged by this site's failure unwind.
    /// Borrow parameters precede lexical begins; the current instruction's begin has
    /// not succeeded and is excluded. Infallible instructions discharge none.
    #[must_use]
    pub fn failure_ended_borrows(self) -> impl ExactSizeIterator<Item = BorrowIdentity> {
        let mut active = Vec::new();
        if instruction_cleanup(&self.instruction.kind).is_some() {
            active.extend(self.function.function.borrow_parameters.iter().map(|value| value.id));
            for instruction in self.function.function.blocks[self.block_index]
                .instructions
                .iter()
                .take(self.instruction_index)
            {
                match &instruction.kind {
                    raw::InstructionKind::BeginBorrow(definition)
                    | raw::InstructionKind::BeginIndexedBorrow { definition, .. } => {
                        active.push(definition.id);
                    }
                    raw::InstructionKind::EndBorrow { borrow } => {
                        active.retain(|id| id != borrow);
                    }
                    _ => {}
                }
            }
        }
        let owner = self.function.id();
        active.into_iter().rev().map(move |id| BorrowIdentity { owner, index: id.0 })
    }

    /// Checked signed-index access; failure precedes creation of the new borrow.
    #[must_use]
    pub fn indexed_borrow(self) -> Option<VerifiedIndexedBorrow> {
        let raw::InstructionKind::BeginIndexedBorrow { definition, index, cleanup } =
            &self.instruction.kind
        else {
            return None;
        };
        let owner = self.function.id();
        let layouts = &self.function.owner.linear32;
        let container = self.function.function.places.get(definition.place.0 as usize)?;
        let record = layout_type(layouts, container.ty)?;
        Some(VerifiedIndexedBorrow {
            borrow: BorrowIdentity { owner, index: definition.id.0 },
            container: PlaceIdentity { owner, index: definition.place.0 },
            index: ValueIdentity { owner, index: index.0 },
            referent: record.referenced_type()?,
            access: definition.access.into(),
            cleanup: CleanupPlanIdentity { owner, index: cleanup.0 },
            array_length: record.array_length(),
        })
    }

    /// Exact old-referent drop and prepared-value ownership transfer authority.
    #[must_use]
    pub fn borrow_replacement(self) -> Option<VerifiedBorrowReplacement> {
        let raw::InstructionKind::BorrowReplace { borrow, value } = self.instruction.kind else {
            return None;
        };
        let owner = self.function.id();
        let layouts = &self.function.owner.linear32;
        let (referent, _) = borrow_definition(self.function.function, borrow, layouts)?;
        Some(VerifiedBorrowReplacement {
            old_value_drop: VerifiedBorrowReferentDrop {
                borrow: BorrowIdentity { owner, index: borrow.0 },
                referent: layout_type(layouts, referent)?.id(),
            },
            value: ValueIdentity { owner, index: value.0 },
        })
    }
}

pub(super) fn element_type(
    function: &raw::Function,
    place: raw::PlaceId,
    layouts: &VerifiedLayouts,
) -> Option<raw::TypeId> {
    let record = layout_type(layouts, function.places.get(place.0 as usize)?.ty)?;
    let element = record.referenced_type()?;
    match record.category() {
        TypeCategory::FixedArray => Some(raw::TypeId(element.index())),
        TypeCategory::Vec if layout_type(layouts, raw::TypeId(element.index()))?.size() > 0 => {
            Some(raw::TypeId(element.index()))
        }
        _ => None,
    }
}

fn value_owner(function: &raw::Function, value: raw::ValueId) -> Option<raw::PlaceId> {
    function.places.iter().find_map(|place| {
        let matches = match place.kind {
            raw::PlaceKind::Temporary(id) => id == value,
            raw::PlaceKind::Parameter(index) => function
                .parameters
                .get(index as usize)
                .is_some_and(|parameter| parameter.id == value),
            _ => false,
        };
        matches.then_some(place.id)
    })
}

pub(super) fn invalidate_borrow_variants(
    function: &raw::Function,
    borrow: raw::BorrowId,
    variants: &mut [Option<u32>],
) {
    let Some(region) = lexical_borrow_place(function, borrow) else { return };
    for place in &function.places {
        if (place.id == region || is_projection_below(place.id, region, &function.places))
            && let Some(variant) = variants.get_mut(place.id.0 as usize)
        {
            *variant = None;
        }
    }
}

pub(super) fn invalidate_call_variants(
    instruction: &raw::Instruction,
    function: &raw::Function,
    variants: &mut [Option<u32>],
) {
    let raw::InstructionKind::DirectCall { arguments, .. } = &instruction.kind else { return };
    for argument in arguments {
        let raw::CallArgument::Borrow(borrow) = argument else { continue };
        let exclusive = function.borrow_parameters.iter().any(|parameter| {
            parameter.id == *borrow && parameter.access == raw::BorrowAccess::Exclusive
        }) || function.blocks.iter().flat_map(|block| &block.instructions).any(
            |instruction| match &instruction.kind {
                raw::InstructionKind::BeginBorrow(definition)
                | raw::InstructionKind::BeginIndexedBorrow { definition, .. } => {
                    definition.id == *borrow && definition.access == raw::BorrowAccess::Exclusive
                }
                _ => false,
            },
        );
        if exclusive {
            invalidate_borrow_variants(function, *borrow, variants);
        }
    }
}

pub(super) fn verify_consumption(
    instruction: &raw::Instruction,
    function: &raw::Function,
    layouts: &VerifiedLayouts,
    active: &[Option<(raw::PlaceId, raw::BorrowAccess)>],
    errors: &mut Errors,
) {
    for value in consuming_instruction_operands(&instruction.kind) {
        if let Some(owner) = value_owner(function, value)
            && layout_type(layouts, function.places[owner.0 as usize].ty)
                .is_some_and(|ty| ty.drop_kind() != 0)
            && overlaps_active(owner, active, &function.places)
        {
            ownership_error(instruction.span, "instruction consumes a borrowed owner", errors);
        }
    }
}
