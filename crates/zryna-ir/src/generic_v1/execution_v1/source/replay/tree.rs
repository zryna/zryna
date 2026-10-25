//! Source-private canonical static state; the unchanged typed/state issuer rechecks every row.
use super::{Body, Failure, Kind, UntrustedSpan, charge, exhausted, raw, reject, reserve};
mod decomposition;
mod frontier;
mod install;
mod transitions;
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Init {
    Empty,
    Whole,
    Partial,
}
impl Body<'_, '_> {
    pub(super) fn tree_parent(&self, id: u32) -> Option<u32> {
        match self.output.places[id as usize].kind {
            raw::PlaceKind::StructField { base, .. }
            | raw::PlaceKind::EnumPayload { base, .. }
            | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(base),
            _ => None,
        }
    }
    pub(super) fn tree_root(&mut self, mut id: u32) -> Result<u32, Failure> {
        loop {
            charge(self.spent, 1)?;
            let Some(parent) = self.tree_parent(id) else { return Ok(id) };
            id = parent;
        }
    }
    pub(super) fn tree_status(&mut self, id: u32) -> Result<(Init, Option<u32>), Failure> {
        let mut at = id;
        let mut kind = if self.state.partial[at as usize] { Init::Partial } else { Init::Empty };
        let mut cover = None;
        loop {
            charge(self.spent, 1)?;
            if self.state.live[at as usize] {
                kind = Init::Whole;
                cover = Some(at);
            } else if !self.state.partial[at as usize] {
                kind = Init::Empty;
                cover = None;
            }
            let Some(parent) = self.tree_parent(at) else { break };
            at = parent;
        }
        Ok((kind, cover))
    }
    pub(super) fn complete_place(&mut self, id: u32, span: UntrustedSpan) -> Result<(), Failure> {
        if self.tree_status(id)?.0 != Init::Whole {
            return Err(self.error(
                span,
                "ZRYNA-M7007",
                "source static place is incomplete or moved",
            ));
        }
        Ok(())
    }
    fn tree_contains(&mut self, parent: u32, mut child: u32) -> Result<bool, Failure> {
        loop {
            charge(self.spent, 1)?;
            if parent == child {
                return Ok(true);
            }
            let Some(at) = self.tree_parent(child) else { return Ok(false) };
            child = at;
        }
    }
    fn tree_clear(&mut self, id: u32) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        if self.children[id as usize].is_empty() {
            self.state.live[id as usize] = false;
            self.state.partial[id as usize] = false;
            return Ok(());
        }
        for at in 0..self.output.places.len() {
            let at = u32::try_from(at).map_err(|_| Failure::InternalFailure)?;
            if self.tree_contains(id, at)? {
                self.state.live[at as usize] = false;
                self.state.partial[at as usize] = false;
            }
        }
        Ok(())
    }
    pub(super) fn tree_whole(&mut self, id: u32) -> Result<(), Failure> {
        let inherited = self.tree_status(id)?.1.is_some_and(|cover| cover != id);
        self.tree_clear(id)?;
        self.state.live[id as usize] = !inherited;
        Ok(())
    }
    fn tree_erase(&mut self, id: u32) -> Result<(), Failure> {
        self.tree_clear(id)?;
        if self.tree_parent(id).is_none() {
            charge(self.spent, self.state.order.len())?;
            self.state.order.retain(|owner| *owner != id);
        }
        Ok(())
    }
}
