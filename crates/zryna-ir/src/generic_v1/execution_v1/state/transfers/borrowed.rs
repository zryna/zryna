//! Selected child authority is established on this edge only; encoded owner tags stay unchanged.
use super::{Context, Failure, State, raw, reject};
pub(super) fn transfer(
    c: &Context<'_>,
    state: &State,
    bi: usize,
    incoming: &mut [Option<State>],
    queue: &mut Vec<usize>,
    work: &mut usize,
) -> Result<(), Failure> {
    let raw::Terminator::ClosedEnumMatch {
        scrutinee: raw::PlaceOrLoan::Loan(parent),
        mode,
        arms,
        ..
    } = &c.raw().blocks[bi].terminator
    else {
        return Err(reject("borrowed match requires a live exact loan source"));
    };
    if !matches!(mode, raw::MatchMode::SharedLoan | raw::MatchMode::ExclusiveLoan) {
        return Err(Failure::InternalFailure);
    }
    state.read_loan(c, *parent)?;
    for arm in arms {
        let mut selected = state.copy()?;
        if let Some(raw::MatchBinding::Loan(d)) = &arm.binding {
            // Begin visits the prior live order. Its parent/path checks and alias
            // loop are prepaid before the selected candidate becomes live.
            super::super::access::prepay(c, &selected, 2, work)?;
            selected.begin_selected(c, d.id, *parent, arm.ordinal)?;
        } else if arm.binding.is_some() {
            return Err(reject("borrowed edge cannot generate an owned value"));
        }
        // Any ordinary argument access now sees the actual enlarged live order.
        if !arm.edge.arguments.is_empty() {
            super::super::access::prepay(c, &selected, arm.edge.arguments.len(), work)?;
        }
        super::edges::borrowed_transfer(c, selected, (bi, &arm.edge), incoming, queue, work)?;
    }
    Ok(())
}
