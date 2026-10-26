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
        if moving && (place.through.is_some() || place.indexed) {
            return Err(self.error(
                span,
                "ZRYNA-M7007",
                "an affine referent cannot be consumed through a loan",
            ));
        }
        self.access(&place, false, moving, span)?;
        if moving {
            self.move_original(&place, span)?;
        }
        Ok(place.ty)
    }
    pub(super) fn assign(&mut self, target: u32, value: u32, depth: usize) -> Result<(), Failure> {
        let span = self.original.body.expressions[target as usize].span;
        let place = self.assignment_target(target, depth + 1)?;
        if place.indexed {
            return self.assign_indexed(place, value, span, depth);
        }
        let rhs = self.expression(value, Use::Move, depth + 1)?;
        self.expect(&rhs, &place.ty, self.original.body.expressions[value as usize].span)?;
        self.assignment_access(&place, span)?;
        self.fill_original(&place)?;
        Ok(())
    }
    fn assignment_target(&mut self, at: u32, depth: usize) -> Result<Place, Failure> {
        super::check_depth(depth)?;
        super::charge(self.spent, 1)?;
        let source = &self.original.body.expressions[at as usize];
        if let super::Expr::Reference { name } = &source.kind {
            for index in (0..self.locals.len()).rev() {
                super::charge(
                    self.spent,
                    super::add(self.locals[index].name.len().min(name.text.len()), 1)?,
                )?;
                if self.locals[index].name == name.text {
                    return self.local_place(index);
                }
            }
        }
        self.place(at, depth)
    }
}
