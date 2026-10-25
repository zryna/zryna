//! Complete values transfer; static partial state never grants a whole owner read.
use super::{Context, Failure, State, raw, reject};
mod frontier;
mod install;
mod tree;
pub(in crate::generic_v1::execution_v1::state) use tree::Init;
impl State {
    pub(super) fn value(&self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        let ty = *c
            .checked()
            .values
            .get(id as usize)
            .ok_or_else(|| reject("state operand has a foreign value"))?;
        if c.affine(ty)? {
            if !self.live_values[id as usize] {
                return Err(reject("prepared affine value was moved or is unavailable"));
            }
            let owner = c.checked().places.owners[id as usize]
                .ok_or_else(|| reject("affine value lacks owner"))?;
            self.complete(c, owner)?;
        }
        Ok(())
    }
    pub(super) fn place(&self, c: &Context<'_>, id: u32, moving: bool) -> Result<(), Failure> {
        if c.checked().places.rows.get(id as usize).is_none() {
            return Err(reject("state operand has a foreign place"));
        }
        self.complete(c, id)?;
        self.loan_free(c, id, moving)
    }
    pub(super) fn loan_free(&self, c: &Context<'_>, id: u32, moving: bool) -> Result<(), Failure> {
        super::access::free(c, self, id, moving)
    }
    pub(super) fn read(&self, c: &Context<'_>, source: raw::PlaceOrLoan) -> Result<(), Failure> {
        match source {
            raw::PlaceOrLoan::Place(id) => self.place(c, id, false),
            raw::PlaceOrLoan::Loan(id) => self.read_loan(c, id),
        }
    }
    pub(super) fn take_place(&mut self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        self.place(c, id, true)?;
        self.expose(c, id, true)?;
        self.erase(c, id);
        Ok(())
    }
    pub(super) fn consume(&mut self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        self.value(c, id)?;
        if c.affine(c.checked().values[id as usize])? {
            let owner = c.checked().places.owners[id as usize]
                .ok_or_else(|| reject("affine value lacks exact pending owner"))?;
            self.take_place(c, owner)?;
        }
        Ok(())
    }
    pub(super) fn publish(
        &mut self,
        c: &Context<'_>,
        d: &raw::ValueDefinition,
        tag: Option<u32>,
    ) -> Result<(), Failure> {
        self.publish_at(c, d, tag, None)
    }
    pub(super) fn publish_at(
        &mut self,
        c: &Context<'_>,
        d: &raw::ValueDefinition,
        tag: Option<u32>,
        slot: Option<usize>,
    ) -> Result<(), Failure> {
        let affine = c.affine(d.ty)?;
        let owner = c.checked().places.owners[d.id as usize].filter(|place| {
            affine || !matches!(c.raw().places[*place as usize].kind, raw::PlaceKind::Temporary(_))
        });
        if let Some(place) = owner {
            if self.status(c, place).0 != Init::Empty {
                return Err(reject("pending result would overwrite complete or partial storage"));
            }
            if affine
                && (self.owners.capacity() < self.initialized.len()
                    || self.owners.len() >= self.initialized.len())
            {
                return Err(reject("owner publication lacks prepaid authority capacity"));
            }
        }
        if slot.is_some_and(|slot| slot > self.owners.len()) {
            return Err(reject("move publication lost its exact pending anchor"));
        }
        self.live_values[d.id as usize] = true;
        self.tags[d.id as usize] = tag;
        if let Some(place) = owner {
            self.set_whole(c, place, tag);
            if affine {
                if let Some(slot) = slot {
                    self.owners.insert(slot, place);
                } else {
                    self.owners.push(place);
                }
            }
        }
        Ok(())
    }
    pub(super) fn erase(&mut self, c: &Context<'_>, id: u32) {
        let d = &c.checked().places.decomposition;
        let id = d.canonical(id);
        for at in 0..self.initialized.len() {
            let at = u32::try_from(at).expect("inherited place bound");
            if d.canonical(at) == at && d.contains(id, at) {
                self.initialized[at as usize] = false;
                self.partial[at as usize] = false;
                self.variants[at as usize] = None;
            }
        }
        let row = &c.checked().places.rows[id as usize];
        if let Some(value) = c.checked().places.rows[row.root as usize].value {
            self.live_values[value as usize] = false;
        }
        if row.parent.is_none()
            && let Some(at) = self.owners.iter().position(|owner| *owner == id)
        {
            self.owners.remove(at);
        }
    }
}

impl State {
    pub(in crate::generic_v1::execution_v1::state) fn move_slot(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> Result<Option<usize>, Failure> {
        let row = &c.checked().places.rows[id as usize];
        if row.parent.is_some() || !c.affine(raw::ValueType::Stored(row.ty))? {
            return Ok(None);
        }
        self.owners
            .iter()
            .position(|owner| *owner == row.root)
            .map(Some)
            .ok_or_else(|| reject("whole move lacks its exact pending owner anchor"))
    }
}
