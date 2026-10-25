//! Exclusive replacement commits a complete prepared RHS after all referent checks.
use super::{Context, Failure, State, containers, plan, raw, reject};
pub(super) fn execute(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let raw::Operation::ReplaceLoan { loan, value } = i.operation else {
        return Err(reject("loan replacement requires exact opcode"));
    };
    let target = raw::PlaceOrLoan::Loan(loan);
    state.mutation(c, target)?;
    state.value(c, value)?;
    let tag = state.tags[value as usize];
    let mut candidate = containers::candidate(state, &mut work)?;
    candidate.consume(c, value)?;
    candidate.mutation(c, target)?;
    if let Some(place) = super::super::access::referent_place(c, loan)? {
        // Complete -> complete does not split a Whole parent or reorder its anchor.
        candidate.set_whole(c, place, tag);
    }
    super::super::access::invalidate(c, &mut candidate, loan, &mut work)?;
    plan.retain_drop(index, &[target], true, &mut work)?;
    *state = candidate;
    *spent = work;
    Ok(())
}
