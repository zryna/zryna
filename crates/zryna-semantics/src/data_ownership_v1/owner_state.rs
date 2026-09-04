use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;

use super::type_model::Binding;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnerDelta {
    Registered { owner: raw::PlaceId },
    Renamed { from: raw::PlaceId, to: raw::PlaceId },
    Replaced { prepared: raw::PlaceId, target: raw::PlaceId },
    Transferred { owner: raw::PlaceId },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct OwnerState {
    pub(super) pending: Vec<raw::PlaceId>,
    pub(super) value_owners: BTreeMap<raw::ValueId, raw::PlaceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedStringBranchState {
    pub(super) bindings: BTreeMap<String, Binding>,
    pub(super) owners: OwnerState,
    pub(super) known_bytes: BTreeMap<raw::PlaceId, Option<u64>>,
}

impl OwnerState {
    pub(super) fn pending(&self) -> &[raw::PlaceId] {
        &self.pending
    }

    pub(super) fn contains(&self, owner: raw::PlaceId) -> bool {
        self.pending.contains(&owner)
    }

    pub(super) fn owner(&self, value: raw::ValueId) -> Option<raw::PlaceId> {
        self.value_owners.get(&value).copied()
    }

    pub(super) fn register(
        &mut self,
        value: raw::ValueId,
        owner: raw::PlaceId,
    ) -> Option<OwnerDelta> {
        if self.value_owners.contains_key(&value)
            || self.value_owners.values().any(|candidate| *candidate == owner)
            || self.pending.contains(&owner)
        {
            return None;
        }
        self.pending.push(owner);
        self.value_owners.insert(value, owner);
        Some(OwnerDelta::Registered { owner })
    }

    pub(super) fn register_parameter(&mut self, owner: raw::PlaceId) -> Option<OwnerDelta> {
        if self.pending.contains(&owner)
            || self.value_owners.values().any(|candidate| *candidate == owner)
        {
            return None;
        }
        self.pending.push(owner);
        Some(OwnerDelta::Registered { owner })
    }

    pub(super) fn rehome_move_result(
        &mut self,
        value: raw::ValueId,
        from: raw::PlaceId,
    ) -> Option<OwnerDelta> {
        let to = self.owner(value)?;
        if from == to {
            return None;
        }
        let from_slot = self.pending.iter().position(|place| *place == from)?;
        let to_slot = self.pending.iter().position(|place| *place == to)?;
        self.pending.remove(to_slot);
        let from_slot = from_slot - usize::from(to_slot < from_slot);
        self.pending[from_slot] = to;
        Some(OwnerDelta::Renamed { from, to })
    }

    pub(super) fn rename(&mut self, value: raw::ValueId, to: raw::PlaceId) -> Option<OwnerDelta> {
        let from = self.owner(value)?;
        if from == to || self.pending.contains(&to) {
            return None;
        }
        let slot = self.pending.iter().position(|place| *place == from)?;
        self.pending[slot] = to;
        self.value_owners.remove(&value);
        Some(OwnerDelta::Renamed { from, to })
    }

    pub(super) fn replace(
        &mut self,
        value: raw::ValueId,
        target: raw::PlaceId,
    ) -> Option<OwnerDelta> {
        let prepared = self.owner(value)?;
        if prepared == target {
            return None;
        }
        let target_slot = self.pending.iter().position(|place| *place == target)?;
        let prepared_slot = self.pending.iter().position(|place| *place == prepared)?;
        self.pending[prepared_slot] = target;
        self.pending.remove(target_slot);
        self.value_owners.remove(&value);
        Some(OwnerDelta::Replaced { prepared, target })
    }

    pub(super) fn transfer(&mut self, value: raw::ValueId) -> Option<OwnerDelta> {
        let owner = self.owner(value)?;
        let slot = self.pending.iter().position(|place| *place == owner)?;
        self.pending.remove(slot);
        self.value_owners.remove(&value);
        Some(OwnerDelta::Transferred { owner })
    }

    pub(super) fn transfer_batch(&mut self, values: &[raw::ValueId]) -> Option<Vec<OwnerDelta>> {
        let mut consumed = std::collections::BTreeSet::new();
        let mut deltas = Vec::with_capacity(values.len());
        for value in values {
            let owner = self.owner(*value)?;
            if !self.contains(owner) || !consumed.insert(owner) {
                return None;
            }
            deltas.push(OwnerDelta::Transferred { owner });
        }
        self.pending.retain(|owner| !consumed.contains(owner));
        self.value_owners.retain(|_, owner| !consumed.contains(owner));
        Some(deltas)
    }

    pub(super) fn consume_owner(&mut self, owner: raw::PlaceId) -> Option<OwnerDelta> {
        let slot = self.pending.iter().position(|place| *place == owner)?;
        self.pending.remove(slot);
        self.value_owners.retain(|_, candidate| *candidate != owner);
        Some(OwnerDelta::Transferred { owner })
    }
}

pub(super) fn apply_owner_delta<T>(known: &mut BTreeMap<raw::PlaceId, T>, delta: OwnerDelta) {
    match delta {
        OwnerDelta::Registered { .. } => {}
        OwnerDelta::Renamed { from, to } => {
            if let Some(bytes) = known.remove(&from) {
                known.insert(to, bytes);
            }
        }
        OwnerDelta::Replaced { prepared, target } => {
            known.remove(&target);
            if let Some(bytes) = known.remove(&prepared) {
                known.insert(target, bytes);
            }
        }
        OwnerDelta::Transferred { owner } => {
            known.remove(&owner);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedVecBranchState {
    pub(super) bindings: BTreeMap<String, Binding>,
    pub(super) owners: OwnerState,
    pub(super) known_string_bytes: BTreeMap<raw::PlaceId, u64>,
}
