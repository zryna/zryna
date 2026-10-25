//! Actual eight-array copy kernel: no source, owner program or runtime authority is issued.
use super::{State, budget, candidate};
mod preparation;
#[test]
fn actual_candidate_copy_prepays_its_own_exact_limit_extra_overflow_and_retry() {
    let state = State {
        initialized: vec![true, false, true],
        partial: vec![false; 3],
        live_values: vec![true, false],
        live_loans: vec![true],
        owners: vec![0, 2],
        loan_order: vec![0],
        variants: vec![None; 3],
        tags: vec![None; 2],
    };
    let mut cost = 0;
    assert_eq!(candidate(&state, &mut cost).expect("actual complete eight-array copy"), state);
    assert!(
        cost >= 2 * (3 + 3 + 2 + 1 + 2 + 1 + 3 + 2) + 10,
        "all reads/new slots and reservations are paid by this phase"
    );
    let limit = budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    assert_eq!(candidate(&state, &mut exact).expect("exact remaining own-phase credit"), state);
    assert_eq!(exact, limit);
    let mut short = limit - cost + 1;
    let before = short;
    assert!(candidate(&state, &mut short).is_err());
    assert_eq!(short, before);
    let mut overflow = usize::MAX;
    assert!(candidate(&state, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    assert_eq!(candidate(&state, &mut retry).expect("pristine actual-copy retry"), state);
    assert_eq!(retry, cost);
    assert_eq!(
        state.initialized,
        vec![true, false, true],
        "source was never mutated by candidate retention"
    );
}

#[test]
fn empty_owner_candidate_reserves_result_before_commit() {
    let state = State {
        initialized: vec![],
        partial: vec![],
        live_values: vec![],
        live_loans: vec![],
        owners: vec![],
        loan_order: vec![],
        variants: vec![],
        tags: vec![],
    };
    let mut spent = 0;
    let mut prepared = candidate(&state, &mut spent).expect("prepare spare result owner");
    assert!(prepared.owners.capacity() >= 1, "publish must not allocate after commit");
    let capacity = prepared.owners.capacity();
    prepared.owners.push(0);
    assert_eq!(prepared.owners.capacity(), capacity, "publication uses prepared owner slot");
    assert!(state.owners.is_empty(), "candidate preparation cannot publish in original state");
    assert_eq!(spent, 10, "eight empty reservations plus result slot and reserve are prepaid");
}

#[test]
fn candidate_copy_pays_full_retained_owner_and_loan_authority_slots() {
    let state = State {
        initialized: vec![false; 4],
        partial: vec![false; 4],
        live_values: vec![false; 2],
        live_loans: vec![false; 3],
        owners: vec![],
        loan_order: vec![],
        variants: vec![None; 4],
        tags: vec![None; 2],
    };
    let mut cost = 0;
    let prepared = candidate(&state, &mut cost).expect("actual full-capacity candidate copy");
    assert!(prepared.owners.capacity() >= 4, "retain every authority-bounded owner slot");
    assert!(prepared.loan_order.capacity() >= 3, "retain every authority-bounded loan slot");
    assert!(
        cost >= 19 + 26 + 10,
        "19 source rows,26 retained slots,eight copies and result preparation"
    );
}
