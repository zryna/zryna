use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;

use super::constructor_resources::ConstructorStorage;
use super::preparation_plan::PreparationFacts;
use super::{Binding, ConstructorValueTypes, OwnerState, PrivateOwnedAggregateLowerer};

pub(super) struct StructuredCheckpoint {
    bindings: BTreeMap<String, Binding>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved: BTreeSet<raw::PlaceId>,
    partial: BTreeSet<raw::PlaceId>,
    places: Vec<raw::Place>,
    instructions: Vec<raw::Instruction>,
    types: ConstructorValueTypes,
    storage: ConstructorStorage,
    facts: PreparationFacts,
    cleanup: Vec<raw::CleanupPlan>,
    owners: OwnerState,
    counts: [usize; 5],
    reserved: usize,
    next_value: u32,
    next_local: u32,
}

impl StructuredCheckpoint {
    pub(super) fn capture(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> Self {
        Self {
            bindings: lowerer.bindings.clone(),
            projections: lowerer.projections.clone(),
            moved: lowerer.moved_projections.clone(),
            partial: lowerer.partial_roots.clone(),
            places: lowerer.places.clone(),
            instructions: lowerer.instructions.clone(),
            types: lowerer.constructor_types.clone(),
            storage: lowerer.preparation_storage(),
            facts: lowerer.preparation_facts.clone(),
            cleanup: lowerer.cleanup_plans.clone(),
            owners: lowerer.owners.clone(),
            counts: [
                lowerer.cleanup_actions,
                lowerer.aggregate_operands,
                lowerer.aggregate_subobject_moves,
                lowerer.projected_aggregate_clones,
                lowerer.projected_aggregate_assignments,
            ],
            reserved: lowerer.reserved_transitions,
            next_value: lowerer.next_value,
            next_local: lowerer.next_local,
        }
    }

    pub(super) fn restore(self, lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
        lowerer.bindings = self.bindings;
        lowerer.projections = self.projections;
        lowerer.moved_projections = self.moved;
        lowerer.partial_roots = self.partial;
        lowerer.places = self.places;
        lowerer.instructions = self.instructions;
        lowerer.constructor_types = self.types;
        lowerer.constructor_storage = self.storage;
        lowerer.preparation_facts = self.facts;
        lowerer.cleanup_plans = self.cleanup;
        lowerer.owners = self.owners;
        [
            lowerer.cleanup_actions,
            lowerer.aggregate_operands,
            lowerer.aggregate_subobject_moves,
            lowerer.projected_aggregate_clones,
            lowerer.projected_aggregate_assignments,
        ] = self.counts;
        lowerer.reserved_transitions = self.reserved;
        lowerer.next_value = self.next_value;
        lowerer.next_local = self.next_local;
    }
}
