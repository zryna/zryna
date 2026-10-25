//! Effective Whole coverage is exposed only along complete touched static ancestries.
use super::{Body, Failure, UntrustedSpan, charge};
impl Body<'_, '_> {
    pub(in crate::generic_v1::execution_v1::source::replay) fn tree_take(
        &mut self,
        id: u32,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        self.complete_place(id, span)?;
        self.tree_expose(id, true, span)?;
        self.tree_erase(id)
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn drop_source_tree(
        &mut self,
        id: u32,
        kind: super::Init,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if kind == super::Init::Whole {
            self.tree_expose(id, true, span)?;
        }
        self.instruction(None, super::raw::Operation::DropPlace { place: id }, span)?;
        self.tree_erase(id)
    }
    pub(super) fn tree_expose(
        &mut self,
        id: u32,
        initialized: bool,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let mut covering = id;
        let mut at = id;
        loop {
            charge(self.spent, 1)?;
            if self.state.live[at as usize] || !self.state.partial[at as usize] {
                covering = at;
            }
            let Some(parent) = self.tree_parent(at) else { break };
            at = parent;
        }
        at = covering;
        while at != id {
            self.expand_children(at, span)?;
            let count = self.children[at as usize].len();
            charge(self.spent, count)?;
            let mut next = None;
            for index in 0..count {
                let child = self.children[at as usize][index];
                if self.tree_contains(child, id)? {
                    next = Some(child);
                }
            }
            let next = next.ok_or(Failure::InternalFailure)?;
            for index in 0..count {
                let child = self.children[at as usize][index];
                self.state.live[child as usize] = initialized;
                self.state.partial[child as usize] = false;
            }
            self.state.live[at as usize] = false;
            self.state.partial[at as usize] = true;
            at = next;
        }
        Ok(())
    }
    pub(super) fn tree_promote(&mut self, id: u32) -> Result<(), Failure> {
        let mut highest = id;
        let mut at = id;
        while let Some(parent) = self.tree_parent(at) {
            charge(self.spent, self.children[parent as usize].len() + 1)?;
            if !self.expanded[parent as usize]
                || !self.children[parent as usize]
                    .iter()
                    .all(|child| self.state.live[*child as usize])
            {
                break;
            }
            self.state.live[parent as usize] = true;
            self.state.partial[parent as usize] = false;
            highest = parent;
            at = parent;
        }
        self.tree_whole(highest)
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn normalize_empty_ancestors(
        &mut self,
        id: u32,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let mut at = self.tree_parent(id);
        while let Some(parent) = at {
            charge(self.spent, 1)?;
            if self.state.partial[parent as usize] && !self.has_initialized_storage(parent)? {
                // Already-empty frontier: this existing operation erases only core
                // structural Partial/pending state, never a remaining Copy sibling.
                self.drop_place(parent, span)?;
            }
            at = self.tree_parent(parent);
        }
        Ok(())
    }
}
