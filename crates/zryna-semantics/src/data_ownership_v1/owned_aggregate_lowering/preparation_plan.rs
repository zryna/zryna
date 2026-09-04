use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_constructor_plan::ConstructorKind;
use super::super::owner_state::OwnerDelta;
use super::super::type_model::OwnedAggregatePlace;
use super::super::{OwnerState, Ty};
use super::operand_decisions::{ProjectionOperation, ReferenceDecision};
use super::preparation_state::{Checkpoint, PlannedPlace};
use super::projection_topology::ProjectionDescriptor;

pub(super) enum Leaf<'f> {
    Bool(bool),
    I32(i32),
    String { bytes: &'f [u8], cleanup: raw::CleanupPlanId },
    Reference(ReferenceDecision),
    Projection { source: OwnedAggregatePlace, operation: ProjectionOperation },
    StringClone { source: OwnedAggregatePlace, cleanup: raw::CleanupPlanId },
    AggregateClone { source: raw::PlaceId, cleanup: raw::CleanupPlanId, prefix: raw::CleanupPlanId },
}

pub(super) enum Operation<'f> {
    Enter { arity: usize, kind: ConstructorKind, end: usize },
    Release,
    Prefix { id: raw::PlaceId, descriptor: ProjectionDescriptor },
    Cleanup { id: raw::CleanupPlanId, actions: usize, prefix: Option<raw::PlaceId> },
    Leaf(Leaf<'f>),
    Commit { kind: ConstructorKind, values: Vec<raw::ValueId> },
}

pub(super) struct Step<'f> {
    pub(super) operation: Operation<'f>,
    pub(super) ty: Ty,
    pub(super) at: Span,
    pub(super) value: Option<raw::ValueId>,
    pub(super) owners: Vec<OwnerDelta>,
    pub(super) after: Checkpoint,
}

// One flat, affine program. Constructor ranges borrow no independently consumable subplan.
pub(super) struct PreparationPlan<'f> {
    pub(super) start: Checkpoint,
    pub(super) steps: Vec<Step<'f>>,
    pub(super) result: raw::ValueId,
    pub(super) result_type: Ty,
    pub(super) owners: OwnerState,
    pub(super) projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    pub(super) moved: BTreeSet<raw::PlaceId>,
    pub(super) partial: BTreeSet<raw::PlaceId>,
    pub(super) places: Vec<PlannedPlace>,
    pub(super) visits: usize,
}
