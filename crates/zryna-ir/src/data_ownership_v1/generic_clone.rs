//! Canonical non-handle structural clone and its recursive destination frontier.

use super::{
    BorrowIdentity, CleanupPlanIdentity, Errors, LayoutTypeId, OwnershipFlow, PlaceIdentity,
    TypeCategory, ValueIdentity, VerifiedDropAction, VerifiedInstruction, VerifiedLayouts,
    error_at, layout_type, raw,
};
use std::collections::BTreeSet;

/// Exact retained operand role of the canonical clone operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedGenericCloneSource {
    /// Complete initialized root or exact static subobject read directly.
    Place(PlaceIdentity),
    /// Active exact referent, including indexed elements and call-frame parameters.
    Borrow(BorrowIdentity),
}

/// One sealed generic clone, retaining its complete source and issuing a distinct result owner.
///
/// ```compile_fail
/// use zryna_ir::data_ownership_v1::VerifiedGenericClone;
/// let forged = VerifiedGenericClone {};
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VerifiedGenericClone<'a> {
    instruction: VerifiedInstruction<'a>,
    source: VerifiedGenericCloneSource,
    destination: raw::PlaceId,
    result: raw::ValueId,
    ty: LayoutTypeId,
    cleanup: raw::CleanupPlanId,
    prefix_cleanup: raw::CleanupPlanId,
}

impl<'a> VerifiedInstruction<'a> {
    /// Returns the canonical non-handle clone authority, not a legacy clone interpretation.
    #[must_use]
    pub fn generic_clone(self) -> Option<VerifiedGenericClone<'a>> {
        let owner = self.function.id();
        let (source, cleanup, prefix_cleanup) = match self.instruction.kind {
            raw::InstructionKind::GenericClonePlace { place, cleanup, prefix_cleanup } => (
                VerifiedGenericCloneSource::Place(PlaceIdentity { owner, index: place.0 }),
                cleanup,
                prefix_cleanup,
            ),
            raw::InstructionKind::GenericCloneBorrow { borrow, cleanup, prefix_cleanup } => (
                VerifiedGenericCloneSource::Borrow(BorrowIdentity { owner, index: borrow.0 }),
                cleanup,
                prefix_cleanup,
            ),
            _ => return None,
        };
        let result = self.instruction.result?;
        let destination =
            super::unique_temporary_owner(self.function.function, result.id, result.ty)?;
        Some(VerifiedGenericClone {
            instruction: self,
            source,
            destination,
            result: result.id,
            ty: layout_type(&self.function.owner.linear32, result.ty)?.id(),
            cleanup,
            prefix_cleanup,
        })
    }

    /// Returns recursive destination-prefix cleanup followed by pre-existing live owners.
    #[must_use]
    pub fn generic_clone_prefix_failure_drop_actions(
        self,
    ) -> impl ExactSizeIterator<Item = VerifiedDropAction> {
        let actions = self.generic_clone().and_then(|clone| {
            super::derive_state_before(
                self.function.function,
                &self.function.owner.linear32,
                self.block_index,
                self.instruction_index,
            )
            .map(|(states, variants)| {
                super::sealed_drop_actions(
                    self.function.id(),
                    self.function.function,
                    clone.prefix_cleanup,
                    &states,
                    &variants,
                )
            })
        });
        actions.unwrap_or_default().into_iter()
    }
}

impl<'a> VerifiedGenericClone<'a> {
    /// Exact retained source role; an indexed referent never becomes a fabricated place.
    #[must_use]
    pub const fn source(self) -> VerifiedGenericCloneSource {
        self.source
    }
    /// Distinct destination, published only after complete preparation succeeds.
    #[must_use]
    pub const fn destination(self) -> PlaceIdentity {
        PlaceIdentity { owner: self.instruction.function.id(), index: self.destination.0 }
    }
    /// Exact newly owned result value.
    #[must_use]
    pub const fn result(self) -> ValueIdentity {
        ValueIdentity { owner: self.instruction.function.id(), index: self.result.0 }
    }
    /// Exact sealed root type shared by source and result.
    #[must_use]
    pub const fn ty(self) -> LayoutTypeId {
        self.ty
    }
    /// Failure before any destination resource has been acquired.
    #[must_use]
    pub const fn cleanup(self) -> CleanupPlanIdentity {
        CleanupPlanIdentity { owner: self.instruction.function.id(), index: self.cleanup.0 }
    }
    /// Failure after destination traversal has begun, including an empty allocated Vec.
    #[must_use]
    pub const fn prefix_cleanup(self) -> CleanupPlanIdentity {
        CleanupPlanIdentity { owner: self.instruction.function.id(), index: self.prefix_cleanup.0 }
    }
    /// Symbolic type-directed traversal and failure frontier, not a runtime progress receipt.
    #[must_use]
    pub const fn frontier(self) -> VerifiedGenericCloneFrontier<'a> {
        VerifiedGenericCloneFrontier { clone: self }
    }
}

/// Exact operation-bound recursive clone protocol over the retained layout graph.
///
/// Construction visits Struct fields in declaration order, fixed-array and Vec elements in
/// ascending order, and only the runtime-active Enum payload. Each frame records completed
/// children plus at most one partial child. Failure first unwinds that partial child, then
/// completed children in reverse, and finally releases that frame's acquired Vec storage.
/// String failure publishes no String child. Copy children require no owned cleanup.
///
/// A Vec indirection may revisit a type but only traverses finite runtime values. The static
/// graph below visits each identity once; it neither unfolds recursion nor asserts runtime
/// progress, allocation success, initialized counts, enum tags, or execution receipts.
///
/// ```compile_fail
/// use zryna_ir::data_ownership_v1::VerifiedGenericCloneFrontier;
/// let forged = VerifiedGenericCloneFrontier {};
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VerifiedGenericCloneFrontier<'a> {
    clone: VerifiedGenericClone<'a>,
}

impl<'a> VerifiedGenericCloneFrontier<'a> {
    /// Clone whose distinct destination and unique failure site bind this frontier.
    #[must_use]
    pub const fn clone_operation(self) -> VerifiedGenericClone<'a> {
        self.clone
    }
    /// Reachable type records in canonical identity order, without recursive expansion.
    ///
    /// Child identities, array lengths and variant ordinals come only from these sealed records.
    #[must_use]
    pub fn types(self) -> impl ExactSizeIterator<Item = zryna_layout::VerifiedType<'a>> {
        let layouts = &self.clone.instruction.function.owner.linear32;
        let reachable = reachable_types(self.clone.ty, layouts).unwrap_or_default();
        reachable
            .into_iter()
            .filter_map(|ty| layouts.type_by_id(ty))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

pub(super) fn classify_program(program: &raw::Program, layouts: &VerifiedLayouts) -> Vec<bool> {
    let has_generic_clone =
        program.modules.iter().flat_map(|module| &module.functions).any(|function| {
            function.blocks.iter().flat_map(|block| &block.instructions).any(|instruction| {
                matches!(
                    instruction.kind,
                    raw::InstructionKind::GenericClonePlace { .. }
                        | raw::InstructionKind::GenericCloneBorrow { .. }
                        | raw::InstructionKind::GenericMoveFromPlace { .. }
                        | raw::InstructionKind::GenericReplacePlace { .. }
                )
            })
        });
    if !has_generic_clone {
        return Vec::new();
    }
    let mut parents = vec![Vec::new(); layouts.types().len()];
    let mut invalid = vec![false; parents.len()];
    let mut pending = Vec::new();
    for record in layouts.types() {
        let index = record.id().index() as usize;
        let mut add_child = |child: LayoutTypeId| parents[child.index() as usize].push(index);
        match record.category() {
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
            TypeCategory::Struct => {
                for field in record.fields() {
                    add_child(field.ty());
                }
            }
            TypeCategory::Enum => {
                for payload in record.variants().iter().filter_map(|variant| variant.payload()) {
                    add_child(payload);
                }
            }
            TypeCategory::FixedArray | TypeCategory::Vec => {
                if let Some(child) = record.referenced_type() {
                    add_child(child);
                    invalid[index] = record.category() == TypeCategory::Vec
                        && layouts.type_by_id(child).is_none_or(|element| element.size() == 0);
                } else {
                    invalid[index] = true;
                }
            }
            TypeCategory::Shared | TypeCategory::Weak => invalid[index] = true,
        }
        if invalid[index] {
            pending.push(index);
        }
    }
    // Reverse reachability propagates every forbidden descendant through indirection cycles.
    // Each identity is enqueued once and each sealed edge is examined once.
    while let Some(child) = pending.pop() {
        for &parent in &parents[child] {
            if !invalid[parent] {
                invalid[parent] = true;
                pending.push(parent);
            }
        }
    }
    layouts
        .types()
        .map(|record| record.drop_kind() != 0 && !invalid[record.id().index() as usize])
        .collect()
}

fn reachable_types(
    root: LayoutTypeId,
    layouts: &VerifiedLayouts,
) -> Option<BTreeSet<LayoutTypeId>> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(ty) = pending.pop() {
        if !reached.insert(ty) {
            continue;
        }
        let record = layouts.type_by_id(ty)?;
        match record.category() {
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
            TypeCategory::Struct => pending.extend(record.fields().iter().map(|field| field.ty())),
            TypeCategory::Enum => {
                pending.extend(record.variants().iter().filter_map(|variant| variant.payload()));
            }
            TypeCategory::FixedArray => pending.push(record.referenced_type()?),
            TypeCategory::Vec => {
                let element = record.referenced_type()?;
                if layouts.type_by_id(element)?.size() == 0 {
                    return None;
                }
                pending.push(element);
            }
            TypeCategory::Shared | TypeCategory::Weak => return None,
        }
    }
    Some(reached)
}

pub(super) fn verify_prefix_cleanup(
    instruction: &raw::Instruction,
    owners: &[Option<raw::PlaceId>],
    function: &raw::Function,
    flow: &OwnershipFlow,
    errors: &mut Errors,
) {
    let (source_region, prefix_cleanup) = match instruction.kind {
        raw::InstructionKind::GenericClonePlace { place, prefix_cleanup, .. } => {
            (Some(place), prefix_cleanup)
        }
        raw::InstructionKind::GenericCloneBorrow { borrow, prefix_cleanup, .. } => {
            (super::lexical_borrow_place(function, borrow), prefix_cleanup)
        }
        _ => return,
    };
    let Some(plan) = function.cleanup_plans.get(prefix_cleanup.0 as usize) else { return };
    let destination =
        instruction.result.and_then(|result| owners.get(result.id.0 as usize).copied().flatten());
    let Some(destination) = destination.filter(|destination| {
        source_region.is_none_or(|source| super::root_place(source, function) != *destination)
    }) else {
        errors.push(error_at(
            "ZRYNA-I3012",
            plan.span,
            "generic clone prefix cleanup lacks its distinct destination owner",
            "bind the clone result to one exact temporary owner distinct from its retained source",
        ));
        return;
    };
    let expected = std::iter::once(raw::DropAction::DropGenericCloneInitializedPrefix(destination))
        .chain(flow.pending.iter().rev().copied().map(raw::DropAction::DropPlace))
        .collect::<Vec<_>>();
    if plan.actions != expected {
        errors.push(error_at(
            "ZRYNA-I3013",
            plan.span,
            "generic clone prefix cleanup does not unwind its destination before live roots",
            "unwind the exact recursive destination frontier first, then all pre-existing owners in reverse order",
        ));
    }
}
