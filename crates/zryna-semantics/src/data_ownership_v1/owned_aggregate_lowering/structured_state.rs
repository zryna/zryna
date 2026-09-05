use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::{Binding, OwnerState, PrivateOwnedAggregateLowerer};

#[derive(Clone)]
pub(super) struct JoinState {
    bindings: BTreeMap<String, Binding>,
    owners: OwnerState,
    moved: BTreeSet<raw::PlaceId>,
    partial: BTreeSet<raw::PlaceId>,
    bytes: BTreeMap<raw::PlaceId, u64>,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn join_state(&mut self, at: Span) -> Option<JoinState> {
        if !self.preparation_facts.active_borrows.is_empty()
            || !self.preparation_facts.parameter_borrows.is_empty()
        {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "borrow authority cannot cross a structured ownership edge",
                "end lexical borrows before a branch, loop edge, or continuation",
            );
            return None;
        }
        Some(JoinState {
            bindings: self.bindings.clone(),
            owners: self.owners.clone(),
            moved: self.moved_projections.clone(),
            partial: self.partial_roots.clone(),
            bytes: self.preparation_facts.string_bytes.clone(),
        })
    }

    pub(super) fn restore_join(&mut self, state: &JoinState) {
        self.bindings.clone_from(&state.bindings);
        self.owners.clone_from(&state.owners);
        self.moved_projections.clone_from(&state.moved);
        self.partial_roots.clone_from(&state.partial);
        self.preparation_facts.string_bytes.clone_from(&state.bytes);
        self.preparation_facts.aliases.clear();
        self.preparation_facts.active_borrows.clear();
    }

    pub(super) fn reconcile_join(&mut self, state: &JoinState, at: Span) -> Option<()> {
        let current = self.join_state(at)?;
        if current.bindings != state.bindings
            || current.owners != state.owners
            || current.moved != state.moved
            || current.partial != state.partial
        {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "structured ownership edges have unequal definite states",
                "restore the same live owners and initialization masks without implicit repair",
            );
            return None;
        }
        self.preparation_facts
            .string_bytes
            .retain(|place, bytes| state.bytes.get(place) == Some(bytes));
        Some(())
    }
}
