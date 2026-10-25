//! Exact inherited boundary, overflow and transactional retry controls.

use super::{MAX_OWNER_UNITS, charge};

#[test]
fn exact_owner_boundary_and_first_extra_preserve_completed_work() {
    let mut spent = MAX_OWNER_UNITS - 3;
    charge(&mut spent, 3).expect("exact inherited ceiling");
    assert_eq!(spent, MAX_OWNER_UNITS);
    let failure = charge(&mut spent, 1).expect_err("first extra owner unit");
    let crate::generic_v1::Failure::Diagnostics(errors) = failure else {
        panic!("owning resource diagnostic required");
    };
    assert_eq!(errors[0].code(), "ZRYNA-I3201");
    assert_eq!(spent, MAX_OWNER_UNITS);
    charge(&mut spent, 0).expect("completed-work zero debit");
}

#[test]
fn overflow_and_bad_initial_ledger_reject_atomically() {
    for (initial, units) in [(1, usize::MAX), (usize::MAX, 1), (MAX_OWNER_UNITS + 1, 0)] {
        let mut spent = initial;
        charge(&mut spent, units).expect_err("overflow or already invalid ledger");
        assert_eq!(spent, initial);
    }
}

#[test]
fn failed_precharge_can_retry_without_partial_debit() {
    let mut spent = MAX_OWNER_UNITS - 5;
    charge(&mut spent, 6).expect_err("failed reserve precharge");
    assert_eq!(spent, MAX_OWNER_UNITS - 5);
    charge(&mut spent, 4).expect("smaller retry");
    assert_eq!(spent, MAX_OWNER_UNITS - 1);
    charge(&mut spent, 1).expect("exact final unit");
}
