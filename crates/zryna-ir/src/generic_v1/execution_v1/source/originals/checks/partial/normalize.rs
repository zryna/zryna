//! Exact declared sibling coverage has one canonical empty-subtree spelling.
use super::types::{arity, projected};
use super::{Checker, Failure, Hole, Projection, UntrustedSpan, add, charge, hole_push, path_copy};
impl Checker<'_, '_, '_> {
    pub(super) fn normalize_original(
        &mut self,
        root: usize,
        moved: &[Projection],
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let mut parent = path_copy(moved, self.spent)?;
        while parent.pop().is_some() {
            let mut ty = super::super::clone_type(&self.locals[root].ty, self.spent)?;
            for projection in &parent {
                ty = projected(self.resolver.originals, ty, *projection, self.spent)?;
            }
            let (_, count) = arity(self.resolver.originals, &ty, self.spent)?;
            // Both coverage and compaction compare at most parent.len()+1 items per hole.
            let cost = self.locals[root]
                .holes
                .len()
                .checked_mul(add(parent.len(), 2)?)
                .and_then(|n| n.checked_mul(2))
                .ok_or(Failure::InternalFailure)?;
            charge(self.spent, cost)?;
            let direct_child = |hole: &Hole| {
                hole.path.len() == parent.len() + 1 && hole.path[..parent.len()] == parent
            };
            let covered = self.locals[root].holes.iter().filter(|hole| direct_child(hole)).count();
            // Paths are a disjoint antichain issued by exact typed source projections.
            // Live Copy fields are not holes, and opaque child structure is never guessed.
            if count == 0 || covered != count {
                break;
            }
            self.locals[root].holes.retain(|hole| !direct_child(hole));
            if parent.is_empty() {
                self.locals[root].available = false;
                self.locals[root].moved_at = Some(span);
            } else {
                let path = path_copy(&parent, self.spent)?;
                hole_push(&mut self.locals[root].holes, Hole { path, moved_at: span }, self.spent)?;
            }
        }
        Ok(())
    }
}
