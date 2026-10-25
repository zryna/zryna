//! Mutation invalidates only the exact static referent or conservative dynamic container.
use super::{Context, Failure, State, budget, raw, referent_place, resource_failure};
pub(in crate::generic_v1::execution_v1::state) fn invalidate(
    c: &Context<'_>,
    state: &mut State,
    loan: u32,
    work: &mut usize,
) -> Result<(), Failure> {
    budget::charge(work, 2)?;
    let container = match c.checked().places.loans[loan as usize].dynamic {
        Some(raw::PlaceOrLoan::Place(place)) => Some(place),
        Some(raw::PlaceOrLoan::Loan(parent)) => referent_place(c, parent)?,
        None => return Ok(()),
    };
    clear(c, state, container, work)
}

// Calls may mutate through exclusive arguments before either success or failure.
pub(in crate::generic_v1::execution_v1::state) fn invalidate_call(
    c: &Context<'_>,
    state: &mut State,
    loan: u32,
    work: &mut usize,
) -> Result<(), Failure> {
    if c.checked().places.loans[loan as usize].dynamic.is_some() {
        return invalidate(c, state, loan, work);
    }
    budget::charge(work, 2)?;
    clear(c, state, referent_place(c, loan)?, work)
}
fn clear(
    c: &Context<'_>,
    state: &mut State,
    container: Option<u32>,
    work: &mut usize,
) -> Result<(), Failure> {
    if let Some(place) = container {
        let paths = c
            .raw()
            .places
            .len()
            .checked_add(c.raw().loans.len())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| resource_failure("indexed refinement path overflow"))?;
        budget::charge(
            work,
            c.raw()
                .places
                .len()
                .checked_mul(paths)
                .and_then(|n| n.checked_mul(2))
                .ok_or_else(|| resource_failure("indexed refinement scan overflow"))?,
        )?;
        // Base availability was checked before RHS commit. Reset only complete storage
        // metadata/refinements; root acquisition order and SSA liveness are unchanged.
        state.set_whole(c, place, None);
    }
    Ok(())
}
