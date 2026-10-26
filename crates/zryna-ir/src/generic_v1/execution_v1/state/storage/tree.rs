//! Effective canonical initialization; Whole ancestors cover stale descendant slots.
use super::{Context, Failure, State, raw, reject};
#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::generic_v1::execution_v1::state) enum Init {
    Empty,
    Whole,
    Partial,
}
impl State {
    pub(in crate::generic_v1::execution_v1::state) fn status(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> (Init, Option<u32>) {
        let d = &c.checked().places.decomposition;
        let mut at = d.canonical(id);
        let mut kind = if self.partial[at as usize] { Init::Partial } else { Init::Empty };
        let mut cover = None;
        loop {
            if self.initialized[at as usize] {
                kind = Init::Whole;
                cover = Some(at);
            } else if !self.partial[at as usize] {
                kind = Init::Empty;
                cover = None;
            }
            let Some(parent) = d.parent(at) else { break };
            at = parent;
        }
        (kind, cover)
    }
    pub(in crate::generic_v1::execution_v1::state) fn active(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> Result<(), Failure> {
        let d = &c.checked().places.decomposition;
        let mut at = d.canonical(id);
        while let Some(parent) = d.parent(at) {
            if let raw::PlaceKind::EnumPayload { variant, .. } = c.raw().places[at as usize].kind
                && self.variants[parent as usize] != Some(variant)
            {
                return Err(reject("inactive or unrefined enum payload place access"));
            }
            at = parent;
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::state) fn complete(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> Result<(), Failure> {
        self.active(c, id)?;
        if self.status(c, id).0 != Init::Whole {
            return Err(reject("place is incomplete, uninitialized or already moved"));
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::state) fn tag(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> Option<u32> {
        self.variants[c.checked().places.decomposition.canonical(id) as usize]
    }
    pub(in crate::generic_v1::execution_v1::state) fn set_whole(
        &mut self,
        c: &Context<'_>,
        id: u32,
        tag: Option<u32>,
    ) {
        let d = &c.checked().places.decomposition;
        let id = d.canonical(id);
        let inherited = self.status(c, id).1.is_some_and(|cover| cover != id);
        for at in 0..self.initialized.len() {
            let at = u32::try_from(at).expect("inherited place arena bound");
            if d.canonical(at) == at && d.contains(id, at) {
                self.initialized[at as usize] = false;
                self.partial[at as usize] = false;
                self.variants[at as usize] = None;
            }
        }
        self.initialized[id as usize] = !inherited;
        self.variants[id as usize] = if inherited { None } else { tag };
    }
    // Validate every required split before mutating any initialization or alias state.
    pub(in crate::generic_v1::execution_v1::state) fn expose(
        &mut self,
        c: &Context<'_>,
        id: u32,
        initialized: bool,
    ) -> Result<(), Failure> {
        let d = &c.checked().places.decomposition;
        let target = d.canonical(id);
        // The outermost covering Whole or Empty node overrides descendant scratch states.
        let mut at = target;
        let mut covering = target;
        loop {
            if self.initialized[at as usize] || !self.partial[at as usize] {
                covering = at;
            }
            let Some(parent) = d.parent(at) else { break };
            at = parent;
        }
        let mut at = covering;
        while at != target {
            let children = d.complete(at, self.variants[at as usize])?;
            at = children
                .into_iter()
                .find(|child| d.contains(*child, target))
                .ok_or_else(|| reject("canonical target is missing from complete decomposition"))?;
        }
        at = covering;
        while at != target {
            let next = d
                .complete(at, self.variants[at as usize])?
                .find(|child| d.contains(*child, target))
                .ok_or_else(|| reject("canonical target is absent"))?;
            for child in d.complete(at, self.variants[at as usize])? {
                self.initialized[child as usize] = initialized;
                self.partial[child as usize] = false;
            }
            self.initialized[at as usize] = false;
            self.partial[at as usize] = true;
            at = next;
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1::state) fn promote(
        &mut self,
        c: &Context<'_>,
        id: u32,
    ) -> Result<(), Failure> {
        let d = &c.checked().places.decomposition;
        let mut highest = d.canonical(id);
        let mut at = highest;
        while let Some(parent) = d.parent(at) {
            if !d
                .complete(parent, self.variants[parent as usize])?
                .all(|child| self.initialized[child as usize])
            {
                break;
            }
            self.initialized[parent as usize] = true;
            self.partial[parent as usize] = false;
            highest = parent;
            at = parent;
        }
        let tag = self.variants[highest as usize];
        self.set_whole(c, highest, tag);
        Ok(())
    }
}
