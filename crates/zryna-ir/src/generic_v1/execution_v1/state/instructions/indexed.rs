//! Bounds failure snapshots precede establishment of the new indexed child authority.
use super::{Context, Failure, State, plan, raw, reject};
pub(super) fn begin(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    loan: u32,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<(), Failure> {
    if let raw::LoanOrigin::Indexed { index: value, .. } = c.raw().loans[loan as usize].origin {
        state.value(c, value)?;
        let raw::Operation::BeginLoan { failure: Some(failure), .. } = i.operation else {
            return Err(reject("indexed preparation lacks exact bounds failure"));
        };
        super::failure(c, state, failure, i.site_span, plan, work)?;
        state.begin(c, loan)?;
        plan.operations[index].failure = Some(failure);
    } else {
        state.begin(c, loan)?;
    }
    Ok(())
}
