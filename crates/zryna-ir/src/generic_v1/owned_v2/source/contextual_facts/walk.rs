//! Argument validity is discovered independently; only stored edges determine cycles/facts.
use super::{Budget, Engine, Failure, add, budget, reject};
use super::{syntax::push, terms::Fact};

struct Visit {
    id: u32,
    prefix: usize,
    member: usize,
}
impl Engine<'_, '_> {
    pub(super) fn prove(&mut self, root: u32, credit: &mut Budget) -> Result<Fact, Failure> {
        credit.nominal(1)?;
        if self.expanded == self.terms.len()
            && let Some(fact) = self.terms[root as usize].fact
        {
            return Ok(fact);
        }
        self.expand(credit)?;
        // Every supplied argument is validated as a root, including phantom arguments.
        // Argument dependency cycles impose no storage/affinity conclusion.
        for id in 0..self.terms.len() {
            credit.nominal(1)?;
            self.stored(u32::try_from(id).map_err(|_| Failure::InternalFailure)?, credit)?;
        }
        self.terms[root as usize].fact.ok_or(Failure::InternalFailure)
    }
    fn expand(&mut self, credit: &mut Budget) -> Result<(), Failure> {
        while self.expanded < self.terms.len() {
            credit.nominal(1)?;
            let id = u32::try_from(self.expanded).map_err(|_| Failure::InternalFailure)?;
            let count = self.member_count(id, credit)?;
            // One member cursor; never put all declaration members on the stack.
            for cursor in 0..count {
                credit.nominal(1)?;
                let child = self.member(id, cursor, credit)?;
                push(&mut self.terms[id as usize].members, child, credit)?;
            }
            self.terms[id as usize].state = 1;
            self.expanded += 1;
        }
        Ok(())
    }
    fn stored(&mut self, root: u32, credit: &mut Budget) -> Result<(), Failure> {
        let mut pending = Vec::new();
        self.enter(root, 0, &mut pending, credit)?;
        while let Some(top) = pending.last() {
            let id = top.id;
            let prefix = top.prefix;
            if top.member < self.terms[id as usize].members.len() {
                let child = self.terms[id as usize].members[top.member];
                pending.last_mut().ok_or(Failure::InternalFailure)?.member += 1;
                credit.nominal(1)?;
                if let Some(child) = child {
                    self.enter(child, add(prefix, 1)?, &mut pending, credit)?;
                }
            } else {
                let mut fact = Fact {
                    affine: matches!(self.terms[id as usize].key[0], 2 | 0x30 | 0x31),
                    height: 1,
                };
                for child in self.terms[id as usize].members.iter().flatten() {
                    credit.nominal(1)?;
                    let child = self.terms[*child as usize].fact.ok_or(Failure::InternalFailure)?;
                    fact.affine |= child.affine;
                    fact.height = fact.height.max(add(1, child.height)?);
                }
                height(prefix, fact.height)?;
                let term = &mut self.terms[id as usize];
                term.fact = Some(fact);
                term.state = 2;
                pending.pop();
            }
        }
        Ok(())
    }
    fn enter(
        &mut self,
        id: u32,
        prefix: usize,
        pending: &mut Vec<Visit>,
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        credit.nominal(1)?;
        height(prefix, 1)?;
        let term = &self.terms[id as usize];
        if let Some(fact) = term.fact {
            return height(prefix, fact.height);
        }
        if term.state == 3 {
            return Err(reject("contextual by-value member cycle lacks original acyclic proof"));
        }
        self.terms[id as usize].state = 3;
        // Only stored edges enter this stack, so its inherited path bound is 256.
        push(pending, Visit { id, prefix, member: 0 }, credit)
    }
}
fn height(prefix: usize, child: usize) -> Result<(), Failure> {
    if add(prefix, child)? > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget("contextual stored path exceeds inherited layout traversal depth"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn stored_height_boundary_is_256_independently_of_key_depth() {
        assert!(super::height(0, 256).is_ok());
        assert!(super::height(255, 1).is_ok());
        assert!(super::height(1, 256).is_err());
        assert!(super::height(256, 1).is_err());
        assert!(super::height(usize::MAX, 1).is_err());
    }
}
