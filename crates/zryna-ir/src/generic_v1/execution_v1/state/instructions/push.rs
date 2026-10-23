//! Append growth owns no new result; failure sees the original Vec and prepared RHS.
use super::{Context, Failure, State, failure, plan, raw, reject};
pub(super) fn execute(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let raw::Operation::VecPush { vector, value, failure: id } = i.operation else {
        return Err(reject("append state requires exact VecPush opcode"));
    };
    // Both exact target rechecks are paid before any path/alias traversal.
    super::super::access::prepay(c, state, 2, &mut work)?;
    state.mutation(c, vector)?;
    state.value(c, value)?;
    let mut committed = super::containers::candidate(state, &mut work)?;
    committed.consume(c, value)?;
    committed.mutation(c, vector)?;
    failure(c, state, id, i.site_span, plan, &mut work)?;
    *state = committed;
    plan.operations[index] =
        plan::OperationPlan { destination: None, failure: Some(id), prefix: None, drops: None };
    *spent = work;
    Ok(())
}
