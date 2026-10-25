//! One symbolic body replay proves exact ownership for every repeated latch.
use super::{Checker, Failure, UntrustedSpan, Use, charge, expressions};
impl Checker<'_, '_, '_> {
    pub(super) fn loop_statement(
        &mut self,
        condition: u32,
        body: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        // Formal private-call permissions are distinct from a lexical local loan.
        charge(self.spent, self.locals.len())?;
        if self.locals.iter().enumerate().any(|(index, local)| {
            local.loan.as_ref().is_some_and(|loan| loan.root != index || loan.parent.is_some())
        }) {
            return Err(self.error(span, "ZRYNA-M7007", "lexical loan cannot cross a loop edge"));
        }
        let before = self.snapshot()?;
        let value = self.expression(condition, Use::Move, depth + 1)?;
        let boolean = expressions::scalar(0, self.spent)?;
        self.expect(&value, &boolean, self.original.body.expressions[condition as usize].span)?;
        self.loop_state(&before, span)?;
        let returns = self.block(body, depth + 1)?;
        if !returns {
            self.loop_state(&before, span)?;
        }
        // A while always retains its initial-false path; no constant termination proof.
        self.restore(&before)
    }
    fn loop_state(
        &mut self,
        before: &[super::partial::LocalState],
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let after = self.snapshot()?;
        self.same_original(before, &after, span)
    }
}
