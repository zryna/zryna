use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_constructor_plan::{ConstructorPlanError, ConstructorValueTypes};
use super::super::owned_lowering_resources::{
    CleanupRecipe, CleanupUsage, OwnedCleanupPlanContext,
};
use super::super::{Errors, OwnerState, Ty};
use super::PrivateOwnedAggregateLowerer;
use super::availability::parent_kind;
use super::clone_decisions::CloneUsage;
use super::constructor_resources::{ConstructorStorage, CreditLedgerMut};
use super::projection_topology::{ProjectionDescriptor, ProjectionTopology};
use super::resource_decisions::AggregateUsage;
use super::state::Emission;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Checkpoint {
    pub(super) counts: [usize; 6],
    pub(super) held: [usize; 4],
    pub(super) pending: usize,
    pub(super) cache: (usize, usize),
}

pub(super) struct PlannedPlace {
    pub(super) ty: Ty,
    pub(super) at: Span,
    pub(super) kind: raw::PlaceKind,
}

pub(super) struct PreparationState<'s> {
    pub(super) original_places: &'s [raw::Place],
    pub(super) places: Vec<PlannedPlace>,
    pub(super) projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    pub(super) moved: BTreeSet<raw::PlaceId>,
    pub(super) partial: BTreeSet<raw::PlaceId>,
    pub(super) owners: OwnerState,
    pub(super) counts: [usize; 6],
    pub(super) storage: ConstructorStorage,
    pub(super) transitions: usize,
    pub(super) types: Result<ConstructorValueTypes, ConstructorPlanError>,
    pub(super) cache: (usize, usize),
}

impl PreparationState<'_> {
    pub(super) fn ledger(&mut self) -> CreditLedgerMut<'_> {
        CreditLedgerMut { storage: &mut self.storage, transitions: &mut self.transitions }
    }

    pub(super) fn usage(&self) -> AggregateUsage {
        let [held_values, held_places, held_operands] = self.storage.counts();
        AggregateUsage {
            values: self.counts[0],
            places: self.counts[1],
            transitions: self.counts[2],
            operands: self.counts[3],
            held_values,
            held_places,
            held_operands,
            held_transitions: self.transitions,
        }
    }

    pub(super) fn clone_usage(&self) -> CloneUsage {
        let usage = self.usage();
        CloneUsage {
            values: usage.values.saturating_add(usage.held_values),
            places: usage.places.saturating_add(usage.held_places),
            transitions: usage.transitions,
            reserved_transitions: usage.held_transitions,
            cleanup_plans: self.counts[4],
            cleanup_actions: self.counts[5],
            pending: self.owners.pending().len(),
        }
    }

    pub(super) fn checkpoint(&self) -> Checkpoint {
        let [values, places, operands] = self.storage.counts();
        Checkpoint {
            counts: self.counts,
            held: [operands, self.transitions, values, places],
            pending: self.owners.pending().len(),
            cache: self.cache,
        }
    }

    pub(super) fn parent(&self, place: raw::PlaceId) -> Option<raw::PlaceId> {
        let index = place.0 as usize;
        let kind = if index < self.original_places.len() {
            &self.original_places.get(index)?.kind
        } else {
            &self.places.get(index.checked_sub(self.original_places.len())?)?.kind
        };
        parent_kind(kind)
    }

    pub(super) fn emit(&mut self, ty: Ty, at: Span, errors: &mut Errors<'_>) -> Option<Emission> {
        if !self.usage().emit(ty, at, errors) {
            return None;
        }
        let value = raw::ValueId(u32::try_from(self.counts[0]).ok()?);
        let instruction = self.counts[2];
        let values = self.counts[0].checked_add(1)?;
        let transitions = self.counts[2].checked_add(1)?;
        let owner = if ty.is_copy() {
            None
        } else {
            Some((
                raw::PlaceId(u32::try_from(self.counts[1]).ok()?),
                self.counts[1].checked_add(1)?,
            ))
        };
        self.counts[0] = values;
        self.counts[2] = transitions;
        if let Ok(types) = &mut self.types
            && let Err(error) = types.append_predicted(value, ty.ir, instruction)
        {
            self.types = Err(error);
        }
        let mut owners = Vec::new();
        if let Some((owner, places)) = owner {
            self.counts[1] = places;
            self.places.push(PlannedPlace { ty, at, kind: raw::PlaceKind::Temporary(value) });
            owners.push(self.owners.register(value, owner)?);
        }
        Some(Emission { value, owners })
    }

    pub(super) fn reverse_cleanup(
        &mut self,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<(raw::CleanupPlanId, usize)> {
        let usage = CleanupUsage {
            plans: self.counts[4],
            actions: self.counts[5],
            reserved_plans: 0,
            reserved_actions: 0,
        };
        let recipe = CleanupRecipe::reverse(
            &usage,
            self.owners.pending(),
            None,
            OwnedCleanupPlanContext::Aggregate,
            at,
            errors,
        )?;
        let result = (recipe.id, recipe.action_count);
        let plans = self.counts[4].checked_add(1)?;
        let actions = self.counts[5].checked_add(result.1)?;
        self.counts[4] = plans;
        self.counts[5] = actions;
        Some(result)
    }

    pub(super) fn prefix_cleanup(
        &mut self,
        owner: raw::PlaceId,
    ) -> Option<(raw::CleanupPlanId, usize)> {
        let recipe = CleanupRecipe::aggregate_prefix(self.counts[4], self.owners.pending(), owner)?;
        let result = (recipe.id, recipe.action_count);
        let plans = self.counts[4].checked_add(1)?;
        let actions = self.counts[5].checked_add(result.1)?;
        self.counts[4] = plans;
        self.counts[5] = actions;
        Some(result)
    }
}

pub(super) struct PreparationTopology<'p, 's> {
    pub(super) state: &'p mut PreparationState<'s>,
    pub(super) inserted: &'p mut Vec<(raw::PlaceId, ProjectionDescriptor, Checkpoint)>,
}

impl ProjectionTopology for PreparationTopology<'_, '_> {
    fn cached(&self, key: (u32, u8, u32)) -> Option<raw::PlaceId> {
        self.state.projections.get(&key).copied()
    }
    fn used_places(&self) -> usize {
        self.state.counts[1].saturating_add(self.state.storage.counts()[1])
    }
    fn insert(&mut self, descriptor: ProjectionDescriptor) -> Option<raw::PlaceId> {
        let id = raw::PlaceId(u32::try_from(self.state.counts[1]).ok()?);
        self.state.counts[1] = self.state.counts[1].checked_add(1)?;
        self.state.places.push(PlannedPlace {
            ty: descriptor.ty,
            at: descriptor.at,
            kind: descriptor.kind.clone(),
        });
        self.state.projections.insert(descriptor.key, id);
        self.inserted.push((id, descriptor, self.state.checkpoint()));
        Some(id)
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn preparation_checkpoint(&self) -> Checkpoint {
        let [values, places, operands] = self.constructor_storage.counts();
        Checkpoint {
            counts: [
                self.next_value as usize,
                self.places.len(),
                self.instructions.len(),
                self.aggregate_operands,
                self.cleanup_plans.len(),
                self.cleanup_actions,
            ],
            held: [operands, self.reserved_transitions, values, places],
            pending: self.owners.pending().len(),
            cache: self.constructor_types.checkpoint(),
        }
    }
}
