//! Value reads dereference loans; role reads preserve exact loan identity.
use super::places::Place;
use super::{Checker, Closed, Failure, UntrustedSpan, Use, clone_type, symbolic_fact};
impl Checker<'_, '_, '_> {
    pub(super) fn read_local(
        &mut self,
        index: usize,
        span: UntrustedSpan,
        usage: Use,
    ) -> Result<Closed, Failure> {
        if usage == Use::Loan {
            return clone_type(&self.locals[index].ty, self.spent);
        }
        let place = self.local_place(index)?;
        self.read_place(place, span, usage)
    }
    pub(super) fn read_place(
        &mut self,
        place: Place,
        span: UntrustedSpan,
        usage: Use,
    ) -> Result<Closed, Failure> {
        let moving = usage == Use::Move
            && !symbolic_fact(self.resolver.originals, self.owner, &place.ty, self.spent)?.copy;
        if moving && place.through.is_some() {
            return Err(self.error(
                span,
                "ZRYNA-M7007",
                "an affine referent cannot be consumed through a loan",
            ));
        }
        self.access(&place, false, moving, span)?;
        if moving {
            if !place.path.is_empty() {
                return Err(super::reject("original partial-place move state is unfinished"));
            }
            self.locals[place.root].available = false;
        }
        Ok(place.ty)
    }
    pub(super) fn assign(&mut self, target: u32, value: u32, depth: usize) -> Result<(), Failure> {
        let span = self.original.body.expressions[target as usize].span;
        let place = self.place(target, depth + 1)?;
        let rhs = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&rhs, &place.ty, self.original.body.expressions[value as usize].span)?;
        self.access(&place, true, false, span)?;
        Ok(())
    }
}
