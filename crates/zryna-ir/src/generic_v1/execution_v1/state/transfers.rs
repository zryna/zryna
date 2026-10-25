//! Exact predecessor/backedge state; generated bindings are established only on their edge.
use super::{Context, Failure, State, plan, raw, reject};
mod borrowed;
mod edges;
use edges::transfer;

pub(super) fn terminator(
    c: &Context<'_>,
    state: &State,
    bi: usize,
    plan: &mut plan::Plan,
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    super::access::terminator_credit(c, state, &c.raw().blocks[bi].terminator, work)?;
    match &c.raw().blocks[bi].terminator {
        raw::Terminator::Return { value, cleanup } => {
            completed(c, state, work)?;
            let mut exit = state.copy()?;
            if let Some(id) = value {
                exit.consume(c, *id)?;
            }
            plan.claim(c.program, c.function, *cleanup, &exit.actions(c, work)?, None, work)?;
        }
        raw::Terminator::Trap { cleanup, .. } => {
            plan.claim(c.program, c.function, *cleanup, &state.actions(c, work)?, None, work)?;
        }
        raw::Terminator::Jump(edge) => {
            transfer(c, state.copy()?, (bi, edge), None, incoming, queue, work)?;
        }
        raw::Terminator::Branch { condition, yes, no } => {
            state.value(c, *condition)?;
            transfer(c, state.copy()?, (bi, yes), None, incoming, queue, work)?;
            transfer(c, state.copy()?, (bi, no), None, incoming, queue, work)?;
        }
        raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, ty } => {
            if *mode != raw::MatchMode::Value {
                return borrowed::transfer(c, state, bi, incoming, queue, work);
            }
            state.read(c, *scrutinee)?;
            for arm in arms {
                let mut selected = state.copy()?;
                if c.affine(raw::ValueType::Stored(*ty))? {
                    let raw::PlaceOrLoan::Place(place) = scrutinee else {
                        return Err(reject("value match cannot move through a loan"));
                    };
                    selected.take_place(c, *place)?;
                } else if let raw::PlaceOrLoan::Place(place) = scrutinee {
                    selected.variants[*place as usize] = Some(arm.ordinal);
                }
                let binding = match &arm.binding {
                    Some(raw::MatchBinding::Value(d)) => Some(d),
                    None => None,
                    Some(raw::MatchBinding::Loan(_)) => {
                        return Err(reject("loan match binding transition is not implemented yet"));
                    }
                };
                transfer(c, selected, (bi, &arm.edge), binding, incoming, queue, work)?;
            }
        }
        raw::Terminator::WeakUpgradeBranch { weak, success_binding, success, expired, failure } => {
            super::access::prepay(c, state, 1, work)?;
            state.observation(c, *weak)?;
            let span = c.checked().upgrades[bi]
                .ok_or_else(|| reject("upgrade lacks retained source-bound failure site"))?;
            plan.claim(
                c.program,
                c.function,
                *failure,
                &state.actions(c, work)?,
                Some(span),
                work,
            )?;
            transfer(
                c,
                state.copy()?,
                (bi, success),
                Some(success_binding),
                incoming,
                queue,
                work,
            )?;
            transfer(c, state.copy()?, (bi, expired), None, incoming, queue, work)?;
        }
    }
    Ok(())
}

fn completed(c: &Context<'_>, state: &State, work: &mut usize) -> Result<(), Failure> {
    super::budget::charge(work, state.loan_order.len())?;
    if state.loan_order.iter().any(|id| {
        let row = &c.checked().places.loans[*id as usize];
        row.dynamic.is_some()
            || c.raw().loans[*id as usize].region.role == raw::RegionRole::TransientIndexedAccess
    }) {
        return Err(reject("indexed operation must complete before function return"));
    }
    Ok(())
}
