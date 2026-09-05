//! Canonical structural clone protocol for aggregates containing shared or weak handles.

use super::{
    BorrowIdentity, CleanupPlanIdentity, Errors, LayoutTypeId, OwnershipFlow, PlaceIdentity,
    TypeCategory, ValueIdentity, VerifiedDropAction, VerifiedInstruction, VerifiedLayouts,
    error_at, layout_type, raw,
};
use std::collections::BTreeSet;

/// Exact retained operand role of a handle-aware clone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifiedHandleAwareCloneSource {
    /// Complete initialized root or exact static subobject.
    Place(PlaceIdentity),
    /// Active exact referent, including an indexed element.
    Borrow(BorrowIdentity),
}

/// One node in the sealed, type-directed clone recipe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifiedHandleCloneRecipeKind {
    /// Trivially copied scalar.
    Copy,
    /// Independently allocated String clone.
    StringClone,
    /// Fields are prepared in declaration order.
    Struct(Vec<(u32, LayoutTypeId)>),
    /// Only the runtime-active payload is prepared; entries retain source order.
    Enum(Vec<(u32, Option<LayoutTypeId>)>),
    /// Elements are prepared in ascending index order.
    FixedArray {
        /// Exact element type.
        element: LayoutTypeId,
        /// Exact fixed length.
        length: u64,
    },
    /// Runtime elements are prepared in ascending index order.
    VecEach {
        /// Exact runtime element type.
        element: LayoutTypeId,
    },
    /// Increment the exact source shared handle's strong count before publication.
    SharedCountClone,
    /// Increment the exact source weak handle's weak count before publication.
    WeakCountClone,
}

/// Canonical type-identity node in a handle-aware recursive clone recipe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedHandleCloneRecipeNode {
    ty: LayoutTypeId,
    kind: VerifiedHandleCloneRecipeKind,
}

impl VerifiedHandleCloneRecipeNode {
    /// Exact sealed type identity.
    #[must_use]
    pub const fn ty(&self) -> LayoutTypeId {
        self.ty
    }
    /// Exact operation or ordered child frontier for this identity.
    #[must_use]
    pub const fn kind(&self) -> &VerifiedHandleCloneRecipeKind {
        &self.kind
    }
}

/// One sealed handle-aware clone, retaining source, destination and failure authority.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedHandleAwareClone<'a> {
    instruction: VerifiedInstruction<'a>,
    source: VerifiedHandleAwareCloneSource,
    destination: raw::PlaceId,
    result: raw::ValueId,
    ty: LayoutTypeId,
    cleanup: raw::CleanupPlanId,
    prefix_cleanup: raw::CleanupPlanId,
}

impl<'a> VerifiedInstruction<'a> {
    /// Returns the sealed structural handle-clone authority, never a generic-clone relaxation.
    #[must_use]
    pub fn handle_aware_clone(self) -> Option<VerifiedHandleAwareClone<'a>> {
        let owner = self.function.id();
        let (source, cleanup, prefix_cleanup) = match self.instruction.kind {
            raw::InstructionKind::HandleAwareClonePlace { place, cleanup, prefix_cleanup } => (
                VerifiedHandleAwareCloneSource::Place(PlaceIdentity { owner, index: place.0 }),
                cleanup,
                prefix_cleanup,
            ),
            raw::InstructionKind::HandleAwareCloneBorrow { borrow, cleanup, prefix_cleanup } => (
                VerifiedHandleAwareCloneSource::Borrow(BorrowIdentity { owner, index: borrow.0 }),
                cleanup,
                prefix_cleanup,
            ),
            _ => return None,
        };
        let result = self.instruction.result?;
        let destination =
            super::unique_temporary_owner(self.function.function, result.id, result.ty)?;
        Some(VerifiedHandleAwareClone {
            instruction: self,
            source,
            destination,
            result: result.id,
            ty: layout_type(&self.function.owner.linear32, result.ty)?.id(),
            cleanup,
            prefix_cleanup,
        })
    }

    /// Returns partial-destination cleanup followed by the exact pre-existing live roots.
    #[must_use]
    pub fn handle_aware_clone_prefix_failure_drop_actions(
        self,
    ) -> impl ExactSizeIterator<Item = VerifiedDropAction> {
        let actions = self.handle_aware_clone().and_then(|clone| {
            super::derive_state_before(
                self.function.function,
                &self.function.owner.linear32,
                self.function.borrows(),
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

impl<'a> VerifiedHandleAwareClone<'a> {
    /// Exact retained source authority.
    #[must_use]
    pub const fn source(self) -> VerifiedHandleAwareCloneSource {
        self.source
    }
    /// Exact complete owner retaining the source value across any clone failure.
    #[must_use]
    pub fn source_root(self) -> PlaceIdentity {
        let place = match self.source {
            VerifiedHandleAwareCloneSource::Place(place) => raw::PlaceId(place.index),
            VerifiedHandleAwareCloneSource::Borrow(borrow) => self
                .instruction
                .function
                .borrows()
                .region(raw::BorrowId(borrow.index))
                .expect("verified clone borrow region"),
        };
        PlaceIdentity {
            owner: self.instruction.function.id(),
            index: super::root_place(place, self.instruction.function.function).0,
        }
    }
    /// Distinct temporary destination, unpublished until the clone completes.
    #[must_use]
    pub const fn destination(self) -> PlaceIdentity {
        PlaceIdentity { owner: self.instruction.function.id(), index: self.destination.0 }
    }
    /// Exact newly owned result.
    #[must_use]
    pub const fn result(self) -> ValueIdentity {
        ValueIdentity { owner: self.instruction.function.id(), index: self.result.0 }
    }
    /// Exact root type.
    #[must_use]
    pub const fn ty(self) -> LayoutTypeId {
        self.ty
    }
    /// Cleanup before destination acquisition.
    #[must_use]
    pub const fn cleanup(self) -> CleanupPlanIdentity {
        CleanupPlanIdentity { owner: self.instruction.function.id(), index: self.cleanup.0 }
    }
    /// Cleanup after any destination String, Vec storage, or handle count is acquired.
    #[must_use]
    pub const fn prefix_cleanup(self) -> CleanupPlanIdentity {
        CleanupPlanIdentity { owner: self.instruction.function.id(), index: self.prefix_cleanup.0 }
    }
    /// Symbolic backend recipe; it is not an execution or count-increment receipt.
    #[must_use]
    pub const fn frontier(self) -> VerifiedHandleAwareCloneFrontier<'a> {
        VerifiedHandleAwareCloneFrontier { clone: self }
    }
}

/// Finite sealed recipe graph for runtime-active recursive cloning.
///
/// Vec recursion is retained as identity edges and never statically unfolded. A backend must
/// traverse finite runtime values, clone only the active Enum payload, increment each reached
/// Shared/Weak leaf exactly once, and unwind the initialized destination prefix on failure.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedHandleAwareCloneFrontier<'a> {
    clone: VerifiedHandleAwareClone<'a>,
}

impl<'a> VerifiedHandleAwareCloneFrontier<'a> {
    /// Bound clone operation.
    #[must_use]
    pub const fn clone_operation(self) -> VerifiedHandleAwareClone<'a> {
        self.clone
    }
    /// Reachable recipe nodes in canonical type-identity order, without recursive expansion.
    #[must_use]
    pub fn nodes(self) -> impl ExactSizeIterator<Item = VerifiedHandleCloneRecipeNode> {
        recipe(self.clone.ty, &self.clone.instruction.function.owner.linear32)
            .unwrap_or_default()
            .into_iter()
    }
}

pub(super) fn classify_program(program: &raw::Program, layouts: &VerifiedLayouts) -> Vec<bool> {
    let used = program.modules.iter().flat_map(|module| &module.functions).any(|function| {
        function.blocks.iter().flat_map(|block| &block.instructions).any(|instruction| {
            matches!(
                instruction.kind,
                raw::InstructionKind::HandleAwareClonePlace { .. }
                    | raw::InstructionKind::HandleAwareCloneBorrow { .. }
            )
        })
    });
    if !used {
        return Vec::new();
    }
    let mut parents = vec![Vec::new(); layouts.types().len()];
    let mut contains = vec![false; parents.len()];
    let mut invalid = vec![false; parents.len()];
    for record in layouts.types() {
        let index = record.id().index() as usize;
        let mut add_child = |child: LayoutTypeId| parents[child.index() as usize].push(index);
        match record.category() {
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
            TypeCategory::FixedArray => add_child(record.referenced_type().expect("sealed array")),
            TypeCategory::Vec => {
                let element = record.referenced_type().expect("sealed Vec");
                add_child(element);
                invalid[index] = layouts.type_by_id(element).is_none_or(|ty| ty.size() == 0);
            }
            TypeCategory::Shared | TypeCategory::Weak => contains[index] = true,
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
        }
    }
    let mut pending = contains
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect::<Vec<_>>();
    while let Some(child) = pending.pop() {
        for &parent in &parents[child] {
            if !contains[parent] {
                contains[parent] = true;
                pending.push(parent);
            }
        }
    }
    pending.extend(invalid.iter().enumerate().filter_map(|(index, value)| value.then_some(index)));
    while let Some(child) = pending.pop() {
        for &parent in &parents[child] {
            if !invalid[parent] {
                invalid[parent] = true;
                pending.push(parent);
            }
        }
    }
    contains.into_iter().zip(invalid).map(|(has_handle, bad)| has_handle && !bad).collect()
}

fn recipe(
    root: LayoutTypeId,
    layouts: &VerifiedLayouts,
) -> Option<Vec<VerifiedHandleCloneRecipeNode>> {
    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(ty) = pending.pop() {
        if !reached.insert(ty) {
            continue;
        }
        let record = layouts.type_by_id(ty)?;
        match record.category() {
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
            TypeCategory::Bool
            | TypeCategory::I32
            | TypeCategory::String
            | TypeCategory::Shared
            | TypeCategory::Weak => {}
        }
    }
    reached
        .into_iter()
        .map(|ty| {
            let record = layouts.type_by_id(ty)?;
            let kind = match record.category() {
                TypeCategory::Bool | TypeCategory::I32 => VerifiedHandleCloneRecipeKind::Copy,
                TypeCategory::String => VerifiedHandleCloneRecipeKind::StringClone,
                TypeCategory::Struct => VerifiedHandleCloneRecipeKind::Struct(
                    record.fields().iter().map(|field| (field.ordinal(), field.ty())).collect(),
                ),
                TypeCategory::Enum => VerifiedHandleCloneRecipeKind::Enum(
                    record
                        .variants()
                        .iter()
                        .map(|variant| (variant.ordinal(), variant.payload()))
                        .collect(),
                ),
                TypeCategory::FixedArray => VerifiedHandleCloneRecipeKind::FixedArray {
                    element: record.referenced_type()?,
                    length: record.array_length()?,
                },
                TypeCategory::Vec => {
                    VerifiedHandleCloneRecipeKind::VecEach { element: record.referenced_type()? }
                }
                TypeCategory::Shared => VerifiedHandleCloneRecipeKind::SharedCountClone,
                TypeCategory::Weak => VerifiedHandleCloneRecipeKind::WeakCountClone,
            };
            Some(VerifiedHandleCloneRecipeNode { ty, kind })
        })
        .collect()
}

pub(super) fn verify_prefix_cleanup(
    instruction: &raw::Instruction,
    owners: &[Option<raw::PlaceId>],
    function: &raw::Function,
    borrows: &super::BorrowIndex,
    flow: &OwnershipFlow,
    errors: &mut Errors,
) {
    let (source_region, prefix_cleanup) = match instruction.kind {
        raw::InstructionKind::HandleAwareClonePlace { place, prefix_cleanup, .. } => {
            (Some(place), prefix_cleanup)
        }
        raw::InstructionKind::HandleAwareCloneBorrow { borrow, prefix_cleanup, .. } => {
            (borrows.region(borrow), prefix_cleanup)
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
            "handle-aware clone prefix cleanup lacks its distinct destination owner",
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
            "handle-aware clone prefix cleanup does not unwind its destination before live roots",
            "unwind the exact recursive destination frontier first, then all pre-existing owners in reverse order",
        ));
    }
}
