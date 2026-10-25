//! Original symbolic holes identify exact moved subtrees, independent of closed layouts.
use super::places::{Place, Projection};
use super::{Checker, Failure, UntrustedSpan, add, charge, push, reserve};
mod normalize;
mod types;
use types::{arity, projected};
pub(super) struct Hole {
    pub(super) path: Vec<Projection>,
    pub(super) moved_at: UntrustedSpan,
}
pub(super) type LocalState = (bool, Option<UntrustedSpan>, Vec<Hole>);
fn path_copy(path: &[Projection], spent: &mut usize) -> Result<Vec<Projection>, Failure> {
    charge(
        spent,
        path.len().checked_mul(size_of::<Projection>() + 1).ok_or(Failure::InternalFailure)?,
    )?;
    let mut result = reserve(path.len())?;
    result.extend_from_slice(path);
    Ok(result)
}
fn holes_copy(holes: &[Hole], spent: &mut usize) -> Result<Vec<Hole>, Failure> {
    charge(spent, holes.len().checked_mul(size_of::<Hole>() + 1).ok_or(Failure::InternalFailure)?)?;
    let mut result = reserve(holes.len())?;
    for hole in holes {
        result.push(Hole { path: path_copy(&hole.path, spent)?, moved_at: hole.moved_at });
    }
    Ok(result)
}
fn prefix(a: &[Projection], b: &[Projection], spent: &mut usize) -> Result<bool, Failure> {
    charge(spent, add(a.len().min(b.len()), 1)?)?;
    Ok(a.len() <= b.len() && a.iter().zip(b).all(|(a, b)| a == b))
}
fn hole_push(holes: &mut Vec<Hole>, hole: Hole, spent: &mut usize) -> Result<(), Failure> {
    charge(spent, size_of::<Hole>())?;
    push(holes, hole, spent)
}
fn projection_push(
    path: &mut Vec<Projection>,
    projection: Projection,
    spent: &mut usize,
) -> Result<(), Failure> {
    charge(spent, size_of::<Projection>())?;
    push(path, projection, spent)
}
impl Checker<'_, '_, '_> {
    pub(super) fn complete_original(
        &mut self,
        place: &Place,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let local = &self.locals[place.root];
        if !local.available {
            return Err(self.error(span, "ZRYNA-M7007", "original value has already moved"));
        }
        for hole in &local.holes {
            if prefix(&hole.path, &place.path, self.spent)?
                || prefix(&place.path, &hole.path, self.spent)?
            {
                return Err(self.error(
                    span,
                    "ZRYNA-M7007",
                    "original static place is incomplete or already moved",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn move_original(
        &mut self,
        place: &Place,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if place.path.is_empty() {
            self.locals[place.root].available = false;
            self.locals[place.root].moved_at = Some(span);
            self.locals[place.root].holes.clear();
        } else {
            let path = path_copy(&place.path, self.spent)?;
            hole_push(
                &mut self.locals[place.root].holes,
                Hole { path, moved_at: span },
                self.spent,
            )?;
            self.normalize_original(place.root, &place.path, span)?;
        }
        Ok(())
    }
    pub(super) fn fill_original(&mut self, place: &Place) -> Result<(), Failure> {
        let root = place.root;
        let mut holes = std::mem::take(&mut self.locals[root].holes);
        if !self.locals[root].available {
            hole_push(
                &mut holes,
                Hole {
                    path: Vec::new(),
                    moved_at: self.locals[root].moved_at.ok_or(Failure::InternalFailure)?,
                },
                self.spent,
            )?;
        }
        for hole in holes {
            if prefix(&place.path, &hole.path, self.spent)? {
                continue;
            }
            if prefix(&hole.path, &place.path, self.spent)? {
                self.split_empty(root, hole, &place.path)?;
            } else {
                hole_push(&mut self.locals[root].holes, hole, self.spent)?;
            }
        }
        self.locals[root].available = true;
        self.locals[root].moved_at = None;
        Ok(())
    }
    fn split_empty(
        &mut self,
        root: usize,
        hole: Hole,
        target: &[Projection],
    ) -> Result<(), Failure> {
        let mut path = hole.path;
        let mut ty = super::clone_type(&self.locals[root].ty, self.spent)?;
        for projection in &path {
            ty = projected(self.resolver.originals, ty, *projection, self.spent)?;
        }
        for next in &target[path.len()..] {
            let (fields, count) = arity(self.resolver.originals, &ty, self.spent)?;
            charge(self.spent, count)?;
            for ordinal in 0..count {
                let child = if fields {
                    Projection::Field(ordinal)
                } else {
                    Projection::Element(
                        u64::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                    )
                };
                if child != *next {
                    let mut sibling = path_copy(&path, self.spent)?;
                    projection_push(&mut sibling, child, self.spent)?;
                    hole_push(
                        &mut self.locals[root].holes,
                        Hole { path: sibling, moved_at: hole.moved_at },
                        self.spent,
                    )?;
                }
            }
            ty = projected(self.resolver.originals, ty, *next, self.spent)?;
            projection_push(&mut path, *next, self.spent)?;
        }
        Ok(())
    }
    pub(super) fn snapshot(&mut self) -> Result<Vec<LocalState>, Failure> {
        charge(
            self.spent,
            self.locals
                .len()
                .checked_mul(size_of::<LocalState>() + 1)
                .ok_or(Failure::InternalFailure)?,
        )?;
        let mut states = reserve(self.locals.len())?;
        for local in &self.locals {
            states.push((local.available, local.moved_at, holes_copy(&local.holes, self.spent)?));
        }
        Ok(states)
    }
    pub(super) fn restore(&mut self, states: &[LocalState]) -> Result<(), Failure> {
        charge(self.spent, states.len().checked_mul(3).ok_or(Failure::InternalFailure)?)?;
        if self.locals.len() < states.len() {
            return Err(Failure::InternalFailure);
        }
        self.locals.truncate(states.len());
        for (local, state) in self.locals.iter_mut().zip(states) {
            local.available = state.0;
            local.moved_at = state.1;
            local.holes = holes_copy(&state.2, self.spent)?;
        }
        Ok(())
    }
    pub(super) fn same_original(
        &mut self,
        left: &[LocalState],
        right: &[LocalState],
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        if left.len() != right.len() {
            return Err(Failure::InternalFailure);
        }
        charge(self.spent, left.len())?;
        for (a, b) in left.iter().zip(right) {
            charge(self.spent, add(a.2.len(), b.2.len())?)?;
            let mut at = None;
            for (from, into) in [(&a.2, &b.2), (&b.2, &a.2)] {
                for x in from {
                    let mut found = false;
                    for y in into {
                        if prefix(&x.path, &y.path, self.spent)?
                            && prefix(&y.path, &x.path, self.spent)?
                        {
                            found = true;
                        }
                    }
                    if !found {
                        at = Some(x.moved_at);
                    }
                }
            }
            if a.0 != b.0 || at.is_some() {
                let at = at.or(b.1).or(a.1).unwrap_or(span);
                return Err(self.error(
                    at,
                    "ZRYNA-M7007",
                    "original static ownership states differ at control edge",
                ));
            }
        }
        Ok(())
    }
}
