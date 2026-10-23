//! Actual instruction kernels after an incoming replay copy; no source or execution seal.
use super::{Context, State, instructions, plan};
mod fixture;
use fixture::{Case, inputs, program};

#[test]
fn first_string_preparation_uses_prepaid_owner_capacity_after_incoming_copy() {
    check(Case::String);
}
#[test]
fn clone_preserving_its_source_uses_prepaid_new_owner_capacity_after_incoming_copy() {
    check(Case::Clone);
}
#[test]
fn first_begin_loan_uses_prepaid_loan_capacity_after_incoming_copy() {
    check(Case::Loan);
}
fn check(case: Case) {
    let (p, world) = program(case);
    let typed = inputs(&p, &world);
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("actual complete owning replay");
    let c = Context { program: &p, typed: &typed, function: 0 };
    let initial = State::new(&c).expect("actual entry parameters");
    let mut state = initial.copy().expect("actual incoming replay copy");
    let owner_capacity = state.owners.capacity();
    let loan_capacity = state.loan_order.capacity();
    instructions::execute(
        &c,
        &mut state,
        0,
        0,
        &p.functions[0].blocks[0].instructions[0],
        &mut plan,
        &mut 0,
    )
    .expect("actual first String/Clone/BeginLoan transition");
    assert_eq!(
        state.owners.capacity(),
        owner_capacity,
        "actual publication cannot allocate capacity"
    );
    assert_eq!(
        state.loan_order.capacity(),
        loan_capacity,
        "actual loan begin cannot allocate capacity"
    );
    assert!(
        owner_capacity >= p.functions[0].places.len(),
        "copy retains full authority-bounded owner slots"
    );
    assert!(
        loan_capacity >= p.functions[0].loans.len(),
        "copy retains full authority-bounded loan slots"
    );
    match case {
        Case::String => assert_eq!(state.owners, vec![0]),
        Case::Clone => {
            assert_eq!(state.owners, vec![0, 1]);
            assert!(state.initialized[0], "Clone preserves original source");
        }
        Case::Loan => assert!(state.live_loans[0], "actual borrow is established"),
    }
}

#[test]
fn fresh_owner_capacity_rejection_precedes_publication_mutation() {
    let (p, world) = program(Case::String);
    let typed = inputs(&p, &world);
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("pristine replay");
    let c = Context { program: &p, typed: &typed, function: 0 };
    let mut state = State::new(&c).expect("entry").copy().expect("incoming");
    state.owners = Vec::new();
    let before = state.copy().expect("logical snapshot");
    instructions::execute(
        &c,
        &mut state,
        0,
        0,
        &p.functions[0].blocks[0].instructions[0],
        &mut plan,
        &mut 0,
    )
    .expect_err("no unprepared publication allocation");
    assert_eq!(state, before, "reject before live-value/tag/initialization changes");
}
#[test]
fn fresh_loan_capacity_rejection_precedes_liveness_mutation() {
    let (p, world) = program(Case::Loan);
    let typed = inputs(&p, &world);
    let mut plan = plan::derive(&p, &typed, &mut 0).expect("pristine replay");
    let c = Context { program: &p, typed: &typed, function: 0 };
    let mut state = State::new(&c).expect("entry").copy().expect("incoming");
    state.loan_order = Vec::new();
    let before = state.copy().expect("logical snapshot");
    instructions::execute(
        &c,
        &mut state,
        0,
        0,
        &p.functions[0].blocks[0].instructions[0],
        &mut plan,
        &mut 0,
    )
    .expect_err("no unprepared loan allocation");
    assert_eq!(state, before, "reject before live-loan/order changes");
}
