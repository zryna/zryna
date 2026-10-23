//! No fallible preparation or target-proof retention mutates the incoming state.
use super::{Context, Failure, State, plan, raw, reject};
pub(super) fn execute(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<(), Failure> {
    let mut candidate = super::containers::candidate(state, work)?;
    let drops = match i.operation {
        raw::Operation::InitializePlace { place, value } => {
            candidate.install(c, place, value, false, work)?
        }
        raw::Operation::ReplacePlace { place, value } => {
            candidate.install(c, place, value, true, work)?
        }
        raw::Operation::DropPlace { place } => {
            let targets = candidate.drop_targets(c, place, work)?;
            if targets.1 {
                candidate.expose(c, place, true)?;
            }
            candidate.erase(c, place);
            Some(targets)
        }
        _ => return Err(reject("storage replay requires exact place operation")),
    };
    if let Some((targets, whole)) = drops {
        plan.retain_drop(index, &targets, whole, work)?;
    }
    *state = candidate;
    Ok(())
}
