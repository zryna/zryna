//! Fallible preparation keeps the original owner snapshot until the complete commit is checked.
use super::{Context, Failure, State, budget, failure, plan, raw, reject, resource_failure};
#[cfg(test)]
mod tests;

pub(super) fn execute(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<(), Failure> {
    let destination = destination(c, state, i)?;
    let (failure_id, committed) = match &i.operation {
        raw::Operation::VecConstruct { elements, failure, .. } => {
            let mut committed = candidate(state, work)?;
            for value in elements {
                committed.consume(c, *value)?;
            }
            (*failure, committed)
        }
        raw::Operation::SharedConstruct { value, failure, .. } => {
            let mut committed = candidate(state, work)?;
            committed.consume(c, *value)?;
            (*failure, committed)
        }
        raw::Operation::WeakDowngrade { shared, failure } => {
            state.read(c, *shared)?;
            (*failure, candidate(state, work)?)
        }
        _ => {
            return Err(reject("container state requires exact settled preparation opcode"));
        }
    };
    // Derive candidate state offside. Failure still sees every original/pending owner,
    // while a duplicate or unavailable affine operand cannot partially consume state.
    failure(c, state, failure_id, i.site_span, plan, work)?;
    *state = committed;
    plan.operations[index] = plan::OperationPlan {
        destination: Some(destination),
        failure: Some(failure_id),
        prefix: None,
        drops: None,
    };
    Ok(())
}
fn destination(c: &Context<'_>, state: &State, i: &raw::Instruction) -> Result<u32, Failure> {
    let result =
        i.result.as_ref().ok_or_else(|| reject("container preparation result is absent"))?;
    if !c.affine(result.ty)? {
        return Err(reject("container preparation must publish an affine result"));
    }
    let destination = c
        .checked()
        .places
        .owners
        .get(result.id as usize)
        .copied()
        .flatten()
        .ok_or_else(|| reject("container result has no unique pending destination"))?;
    if state.initialized[destination as usize] {
        return Err(reject("container destination is already initialized before preparation"));
    }
    Ok(destination)
}

pub(super) fn candidate(state: &State, spent: &mut usize) -> Result<State, Failure> {
    let mut work = *spent;
    let rows = state
        .initialized
        .len()
        .checked_add(state.partial.len())
        .and_then(|n| n.checked_add(state.live_values.len()))
        .and_then(|n| n.checked_add(state.live_loans.len()))
        .and_then(|n| n.checked_add(state.owners.len()))
        .and_then(|n| n.checked_add(state.loan_order.len()))
        .and_then(|n| n.checked_add(state.variants.len()))
        .and_then(|n| n.checked_add(state.tags.len()))
        .ok_or_else(|| resource_failure("container candidate state row census overflow"))?;
    let spare_owners = state
        .initialized
        .len()
        .checked_sub(state.owners.len())
        .ok_or_else(|| reject("container owner order exceeds authority arena"))?;
    let spare_loans = state
        .live_loans
        .len()
        .checked_sub(state.loan_order.len())
        .ok_or_else(|| reject("container loan order exceeds authority arena"))?;
    let retained = rows
        .checked_add(spare_owners)
        .and_then(|n| n.checked_add(spare_loans))
        .ok_or_else(|| resource_failure("container candidate retained capacity overflow"))?;
    let units = rows
        .checked_add(retained)
        .and_then(|n| n.checked_add(10))
        .ok_or_else(|| resource_failure("container candidate state copy credit overflow"))?;
    // Source reads and full authority-bounded retained capacity (not only live rows)
    // plus all eight fallible copy reserves are paid on this phase's own ledger. The extra
    // result owner slot and its fallible reservation are paid before any commit.
    budget::charge(&mut work, units)?;
    let mut result = state.copy()?;
    result.owners.try_reserve_exact(1).map_err(|_| Failure::AllocationFailure)?;
    *spent = work;
    Ok(result)
}
