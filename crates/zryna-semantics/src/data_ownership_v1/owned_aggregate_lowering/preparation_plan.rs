use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_constructor_plan::ConstructorKind;
use super::super::owned_string_read::StringBytes;
use super::super::owner_state::OwnerDelta;
use super::super::type_model::OwnedAggregatePlace;
use super::super::{OwnerState, Ty};
use super::operand_decisions::{ProjectionOperation, ReferenceDecision};
use super::preparation_state::{Checkpoint, PlannedPlace};
use super::projection_topology::ProjectionDescriptor;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub(super) struct PreparationFacts {
    pub(super) held_cleanup: [usize; 2],
    pub(super) string_bytes: BTreeMap<raw::PlaceId, u64>,
}

impl PreparationFacts {
    pub(super) fn apply(&mut self, delta: OwnerDelta) {
        super::super::owner_state::apply_owner_delta(&mut self.string_bytes, delta);
    }
}

pub(super) enum Leaf<'f> {
    Bool(bool),
    I32(i32),
    String {
        bytes: &'f [u8],
        cleanup: raw::CleanupPlanId,
    },
    Reference(ReferenceDecision),
    Projection {
        source: OwnedAggregatePlace,
        operation: ProjectionOperation,
    },
    StringClone {
        source: OwnedAggregatePlace,
        bytes: StringBytes,
        cleanup: raw::CleanupPlanId,
    },
    StringConcat {
        left: raw::PlaceId,
        right: raw::PlaceId,
        bytes: StringBytes,
        cleanup: raw::CleanupPlanId,
    },
    AggregateClone {
        source: raw::PlaceId,
        cleanup: raw::CleanupPlanId,
        prefix: raw::CleanupPlanId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StringOperation {
    Clone,
    Concat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StringRead {
    pub(super) place: raw::PlaceId,
    pub(super) root: raw::PlaceId,
    pub(super) value: Option<raw::ValueId>,
    pub(super) bytes: StringBytes,
}

pub(super) enum Operation<'f> {
    CallEnter {
        signature: CallSignature,
        end: usize,
        arguments: Vec<raw::ValueId>,
    },
    CallTransfer {
        value: raw::ValueId,
        owner: raw::PlaceId,
    },
    CallRelease,
    CallCommit {
        signature: CallSignature,
        arguments: Vec<raw::ValueId>,
        cleanup: raw::CleanupPlanId,
    },
    StringEnter {
        kind: StringOperation,
        end: usize,
        reads: Vec<StringRead>,
    },
    StringRead(StringRead),
    StringExit,
    Enter {
        arity: usize,
        kind: ConstructorKind,
        end: usize,
    },
    Release,
    Prefix {
        id: raw::PlaceId,
        descriptor: ProjectionDescriptor,
    },
    CloneCapacity {
        aggregate: bool,
    },
    Cleanup {
        id: raw::CleanupPlanId,
        actions: usize,
        prefix: Option<raw::PlaceId>,
    },
    Leaf(Leaf<'f>),
    Commit {
        kind: ConstructorKind,
        values: Vec<raw::ValueId>,
    },
    VecCommit {
        values: Vec<raw::ValueId>,
        cleanup: raw::CleanupPlanId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CallKind {
    String,
    Vec,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CallSignature {
    pub(super) id: raw::FunctionId,
    pub(super) result: Ty,
    pub(super) parameter: Option<Ty>,
    pub(super) kind: CallKind,
    pub(super) bytes: Option<StringBytes>,
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
    pub(super) facts: PreparationFacts,
}
