//! Inherited concat observes exact reads left to right and keeps them valid through preparation.
use super::{Checker, Closed, Failure, UntrustedSpan, Use, charge, clone_type, push};
impl Checker<'_, '_, '_> {
    pub(super) fn concat(
        &mut self,
        args: &[u32],
        has_types: bool,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        if has_types || args.len() != 2 {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "concat requires two String operands and no type arguments",
            ));
        }
        charge(self.spent, args.len())?;
        let mark = self.retained_reads.len();
        let expected = super::expressions::scalar(2, self.spent)?;
        for at in args {
            let site = self.original.body.expressions[*at as usize].span;
            if self.place_expression(*at)? {
                let place = self.place(*at, depth + 1)?;
                self.access(&place, false, false, site)?;
                self.expect(&place.ty, &expected, site)?;
                charge(self.spent, size_of::<super::places::Place>())?;
                push(&mut self.retained_reads, place, self.spent)?;
            } else {
                let actual = self.expression(*at, Use::Observe, depth + 1)?;
                self.expect(&actual, &expected, site)?;
            }
        }
        // Recheck exact retained source reads after the complete RHS preparation.
        while self.retained_reads.len() > mark {
            charge(self.spent, 1)?;
            let read = self.retained_reads.pop().ok_or(Failure::InternalFailure)?;
            self.access(&read, false, false, span)?;
        }
        clone_type(&expected, self.spent)
    }
}
