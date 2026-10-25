//! Prepared installation updates an offside state before any owning commit.
use super::{Context, Failure, Init, State, raw, reject};
impl State {
    pub(in crate::generic_v1::execution_v1::state) fn install(
        &mut self,
        c: &Context<'_>,
        place: u32,
        value: u32,
        replacing: bool,
        work: &mut usize,
    ) -> Result<Option<(Vec<raw::PlaceOrLoan>, bool)>, Failure> {
        let row = &c.checked().places.rows[place as usize];
        let root = row.root;
        if let raw::PlaceKind::Temporary(id) = c.raw().places[place as usize].kind
            && (id != value || replacing || c.affine(raw::ValueType::Stored(row.ty))?)
        {
            return Err(reject("only exact Copy SSA may initialize its addressable temporary"));
        }
        self.active(c, place)?;
        self.loan_free(c, place, true)?;
        let kind = self.status(c, place).0;
        if !replacing && kind != Init::Empty {
            return Err(reject("InitializePlace overwrites complete or partial storage"));
        }
        if replacing && kind == Init::Empty {
            return Err(reject("ReplacePlace targets uninitialized storage"));
        }
        let target_slot = self.owners.iter().position(|place| *place == root);
        let source_slot = c.checked().places.owners[value as usize]
            .and_then(|owner| self.owners.iter().position(|place| *place == owner));
        let inherited_slot = if row.parent.is_none() {
            source_slot.map(|source| {
                source - usize::from(replacing && target_slot.is_some_and(|target| target < source))
            })
        } else if target_slot.is_none() {
            source_slot
        } else {
            None
        };
        let tag = self.tags[value as usize];
        self.consume(c, value)?;
        // Prepared RHS may have consumed the target or an ancestor; recheck after consumption.
        let drops = if replacing {
            Some(self.drop_targets(c, place, work)?)
        } else {
            if self.status(c, place).0 != Init::Empty {
                return Err(reject("initialization target changed during preparation"));
            }
            None
        };
        if replacing && row.parent.is_some() && drops.as_ref().is_some_and(|(_, whole)| *whole) {
            // A complete-to-complete child replacement changes no initialization frontier.
            self.set_whole(c, place, tag);
            return Ok(drops);
        }
        if replacing {
            if self.status(c, place).0 == Init::Whole {
                self.expose(c, place, true)?;
            }
            self.erase(c, place);
        }
        self.expose(c, place, false)?;
        if c.affine(raw::ValueType::Stored(c.checked().places.rows[root as usize].ty))?
            && !self.owners.contains(&root)
        {
            if self.owners.capacity() < self.initialized.len()
                || self.owners.len() >= self.initialized.len()
            {
                return Err(reject("owner installation lacks prepaid capacity"));
            }
            if let Some(slot) = inherited_slot {
                if slot > self.owners.len() {
                    return Err(reject("installation lost its exact RHS pending anchor"));
                }
                self.owners.insert(slot, root);
            } else {
                self.owners.push(root);
            }
        }
        self.set_whole(c, place, tag);
        self.promote(c, place)?;
        if self.initialized[root as usize]
            && let Some(value) = c.checked().places.rows[root as usize].value
        {
            self.live_values[value as usize] = true;
        }
        Ok(drops)
    }
}
