//! V5-only Clone query reuses normalized validity without changing v4 facts or charges.
use super::{Budget, Closed, Engine, Failure, Frame, Originals, add, budget, reserve};
struct Cached {
    key: Vec<u8>,
    available: bool,
    work: usize,
}
pub(in super::super) struct CloneFacts<'a, 'b> {
    originals: &'a Originals<'b>,
    owner: Frame,
    symbolic: bool,
    last: Option<Cached>,
}
impl<'a, 'b> CloneFacts<'a, 'b> {
    pub(in super::super) fn new(
        originals: &'a Originals<'b>,
        owner: Frame,
        symbolic: bool,
    ) -> Self {
        Self { originals, owner, symbolic, last: None }
    }
    pub(in super::super) fn available(
        &mut self,
        ty: &Closed,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        let Closed::Stored(key) = ty else { return Ok(false) };
        if key.len() > 4096 {
            return Err(budget("Clone complete key exceeds 4096 bytes"));
        }
        credit.nominal(add(1, key.len())?)?;
        if let Some(last) = self.last.as_ref().filter(|last| last.key == *key) {
            credit.nominal(last.work)?;
            return Ok(last.available);
        }
        let mut checked = *credit;
        let mut engine = Engine::new(self.originals);
        let root = engine.decode(key, self.owner, self.symbolic, &mut checked)?;
        engine.prove(root, &mut checked)?;
        let available = engine.clone_available(root, &mut checked)?;
        checked.nominal(key.len())?;
        let mut retained = reserve(key.len())?;
        retained.extend_from_slice(key);
        let work = checked
            .nominal_used()
            .checked_sub(credit.nominal_used())
            .ok_or(Failure::InternalFailure)?;
        self.last = Some(Cached { key: retained, available, work });
        *credit = checked;
        Ok(available)
    }
}
struct Visit {
    id: u32,
    next: usize,
}
impl Engine<'_, '_> {
    fn clone_available(&self, root: u32, credit: &mut Budget) -> Result<bool, Failure> {
        let count = self.terms.len();
        credit.nominal(count.checked_mul(2).ok_or(Failure::InternalFailure)?)?;
        let mut complete = reserve(count)?;
        complete.resize(count, None);
        let mut pending = Vec::new();
        super::syntax::push(&mut pending, Visit { id: root, next: 0 }, credit)?;
        while let Some(top) = pending.last() {
            credit.nominal(1)?;
            let id = top.id;
            if top.next < self.terms[id as usize].members.len() {
                let child = self.terms[id as usize].members[top.next];
                pending.last_mut().ok_or(Failure::InternalFailure)?.next += 1;
                if let Some(child) = child
                    && complete[child as usize].is_none()
                {
                    super::syntax::push(&mut pending, Visit { id: child, next: 0 }, credit)?;
                }
            } else {
                let mut available = !matches!(self.terms[id as usize].key[0], 0x30 | 0x31);
                for child in self.terms[id as usize].members.iter().flatten() {
                    credit.nominal(1)?;
                    available &= complete[*child as usize].ok_or(Failure::InternalFailure)?;
                }
                complete[id as usize] = Some(available);
                pending.pop();
            }
        }
        complete[root as usize].ok_or(Failure::InternalFailure)
    }
}

#[cfg(test)]
mod tests;
