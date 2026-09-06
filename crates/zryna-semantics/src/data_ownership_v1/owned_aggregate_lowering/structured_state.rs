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
    aliases: BTreeMap<String, super::preparation_plan::LexicalAlias>,
    reads: BTreeSet<raw::PlaceId>,
    indexed_reads: BTreeSet<raw::PlaceId>,
    active: BTreeMap<raw::BorrowId, (raw::PlaceId, raw::BorrowAccess)>,
    continued: BTreeSet<raw::BorrowId>,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn join_state(&mut self, at: Span) -> Option<JoinState> {
        if self
            .preparation_facts
            .active_borrows
            .keys()
            .any(|id| !self.preparation_facts.continued_borrows.contains(id))
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
            active: self.preparation_facts.active_borrows.clone(),
            continued: self.preparation_facts.continued_borrows.clone(),
            bindings: self.bindings.clone(),
            owners: self.owners.clone(),
            moved: self
                .moved_projections
                .iter()
                .copied()
                .filter(|place| {
                    let mut root = *place;
                    while let Some(parent) = self
                        .places
                        .get(root.0 as usize)
                        .and_then(|place| super::availability::parent_kind(&place.kind))
                    {
                        root = parent;
                    }
                    self.owners.contains(root)
                })
                .collect(),
            partial: self
                .partial_roots
                .iter()
                .copied()
                .filter(|root| self.owners.contains(*root))
                .collect(),
            bytes: self.preparation_facts.string_bytes.clone(),
            aliases: self.preparation_facts.aliases.clone(),
            reads: self.preparation_facts.retained_string_reads.clone(),
            indexed_reads: self.preparation_facts.retained_indexed_reads.clone(),
        })
    }

    pub(super) fn restore_join(&mut self, state: &JoinState) {
        self.bindings.clone_from(&state.bindings);
        self.owners.clone_from(&state.owners);
        self.moved_projections.clone_from(&state.moved);
        self.partial_roots.clone_from(&state.partial);
        self.preparation_facts.string_bytes.clone_from(&state.bytes);
        self.preparation_facts.aliases.clone_from(&state.aliases);
        self.preparation_facts.retained_string_reads.clone_from(&state.reads);
        self.preparation_facts.retained_indexed_reads.clone_from(&state.indexed_reads);
        self.preparation_facts.active_borrows.clone_from(&state.active);
        self.preparation_facts.continued_borrows.clone_from(&state.continued);
    }

    pub(super) fn reconcile_join(&mut self, state: &JoinState, at: Span) -> Option<()> {
        let current = self.join_state(at)?;
        if current.bindings != state.bindings
            || current.owners != state.owners
            || current.moved != state.moved
            || current.partial != state.partial
            || current.aliases != state.aliases
            || current.reads != state.reads
            || current.indexed_reads != state.indexed_reads
            || current.active != state.active
            || current.continued != state.continued
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
