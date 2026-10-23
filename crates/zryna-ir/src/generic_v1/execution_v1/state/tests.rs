//! Local state-kernel controls. These rows never construct execution/source authority.
use super::{State, raw};

#[test]
fn cleanup_reverses_live_loans_then_owners_and_excludes_unpublished_destination() {
    let state = State {
        initialized: vec![true, false, true],
        partial: vec![false; 3],
        live_values: vec![true, false, true],
        live_loans: vec![true, true],
        owners: vec![0, 2],
        loan_order: vec![0, 1],
        variants: vec![None; 3],
        tags: vec![None; 3],
    };
    let expected = vec![
        raw::CleanupAction::EndLoan(1),
        raw::CleanupAction::EndLoan(0),
        raw::CleanupAction::DropPlace(2),
        raw::CleanupAction::DropPlace(0),
    ];
    assert_eq!(state.whole_actions(&mut 0).expect("actual live snapshot"), expected);
    let mut spent = usize::MAX;
    assert!(state.whole_actions(&mut spent).is_err());
    assert_eq!(spent, usize::MAX);
    assert_eq!(state.whole_actions(&mut 0).expect("pristine retry"), expected);
}
