//! Install preserves the core RHS/root anchor law for whole roots and static subobjects.
use super::{Body, Failure, Init, UntrustedSpan, charge, raw};
impl Body<'_, '_> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn install_source(
        &mut self,
        place: u32,
        value: super::super::Value,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let root = self.tree_root(place)?;
        let parent = self.tree_parent(place);
        let kind = self.tree_status(place)?.0;
        let replacing = kind != Init::Empty;
        let source_slot =
            self.owners[value.id as usize].map(|id| self.pending_anchor(id)).transpose()?;
        charge(self.spent, self.state.order.len())?;
        let target_slot = self.state.order.iter().position(|owner| *owner == root);
        let slot = if parent.is_none() {
            source_slot.map(|source| {
                source - usize::from(replacing && target_slot.is_some_and(|target| target < source))
            })
        } else if target_slot.is_none() {
            source_slot
        } else {
            None
        };
        self.consume(value, span)?;
        let operation = if replacing {
            raw::Operation::ReplacePlace { place, value: value.id }
        } else {
            raw::Operation::InitializePlace { place, value: value.id }
        };
        self.instruction(None, operation, span)?;
        if parent.is_some() && kind == Init::Whole {
            return self.tree_whole(place);
        }
        if replacing {
            if kind == Init::Whole {
                self.tree_expose(place, true, span)?;
            }
            self.tree_erase(place)?;
        }
        self.tree_expose(place, false, span)?;
        charge(self.spent, self.state.order.len() + 1)?;
        if !self.copy(self.output.places[root as usize].ty)? && !self.state.order.contains(&root) {
            charge(self.spent, self.state.order.len() + 1)?;
            self.state.order.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
            if let Some(slot) = slot {
                if slot > self.state.order.len() {
                    return Err(Failure::InternalFailure);
                }
                self.state.order.insert(slot, root);
            } else {
                self.state.order.push(root);
            }
        }
        self.tree_whole(place)?;
        self.tree_promote(place)
    }
}
