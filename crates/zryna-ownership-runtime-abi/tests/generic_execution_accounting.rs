//! Seeded arithmetic oracle controls; no execution seal or actual threshold claim.
use sha2::{Digest, Sha256};
use zryna_ownership_runtime_abi::generic_execution_accounting::{CounterState, Fault, Request};
fn state(value: &serde_json::Value) -> CounterState {
    CounterState {
        attempts: value["A"].as_u64().expect("frozen A"),
        live: value["L"].as_u64().expect("frozen L"),
        spent: value["S"].as_u64().expect("frozen S"),
        held: value["H"].as_u64().expect("frozen H"),
    }
}
fn event(id: &str, actual: &mut CounterState) -> Result<(), Fault> {
    if matches!(id, "cleanup-at-total-ceiling" | "invalid-held-debt-release") {
        return actual.release(id == "cleanup-at-total-ceiling");
    }
    let positive = matches!(
        id,
        "positive-attempt-exact"
            | "positive-attempt-first-extra"
            | "valid-allocation-failure-keeps-attempt"
            | "potential-allocation-fault-debt-denied-first"
    );
    let no_owner = matches!(
        id,
        "known-utf8-reject-no-owner"
            | "known-expired-only-forward-credit"
            | "known-refcount-only-forward-credit"
            | "forged-overflow-context"
    );
    let admission = actual.forward(Request {
        attempt: positive,
        new_hold: u64::from(!no_owner),
        temporary_live: positive,
    })?;
    let success = !no_owner && id != "valid-allocation-failure-keeps-attempt";
    actual.complete(admission, success)
}
#[test]
fn fifteen_frozen_seeded_cases_preserve_exact_atomic_counter_and_cleanup_law() {
    let bytes = include_bytes!("generic_execution_accounting/HELD_COUNTER_EXPECTED_FREEZE.json");

    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "578b3f1d8373237b3c92a1267865fefe83243b17fed2d04eaa5098ff51f6faf2"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("pre-implementation numeric oracle");
    let cases = oracle["cases"].as_array().expect("fifteen separate seeded cases");
    assert_eq!(cases.len(), 15);
    for case in cases {
        let id = case["id"].as_str().expect("fixed case");
        let mut actual = state(&case["pre"]);
        let expected = &case["expected"];
        let outcome = event(id, &mut actual);
        let kind = expected["admission"].as_str().expect("independent outcome");
        let result = if kind.starts_with("ADMITTED") {
            Ok(())
        } else if kind.starts_with("DENIED") {
            Err(Fault::Denied)
        } else {
            Err(Fault::Invariant)
        };
        assert_eq!(outcome, result, "{id}: admission/failure class");
        assert_eq!(actual, state(&expected["post"]), "{id}: atomic poststate");
        if !expected["after_cleanup"].is_null() {
            if matches!(id, "known-refcount-only-forward-credit" | "count-success-needs-new-credit")
            {
                actual.release(false).expect("held StrongBegin credit");
                actual.release(true).expect("held StrongFinish credit");
            } else {
                actual.release(actual.live != 0).expect("held cleanup cannot quota-fail");
            }
            assert_eq!(actual, state(&expected["after_cleanup"]), "{id}: exact held cleanup");
        }
    }
}
#[test]
fn impossible_requested_sum_denies_without_mutation_then_pristine_retry_succeeds() {
    let mut actual = CounterState::default();
    let before = actual;
    assert!(matches!(
        actual.forward(Request { attempt: false, new_hold: u64::MAX, temporary_live: false }),
        Err(Fault::Denied)
    ));
    assert_eq!(actual, before);
    let admitted = actual
        .forward(Request { attempt: false, new_hold: 1, temporary_live: false })
        .expect("pristine empty owner");
    actual.complete(admitted, true).expect("successful empty owner");
    actual.release(false).expect("held empty release");
    assert_eq!(actual, CounterState { spent: 2, ..before });
}
#[test]
fn interleaved_reservation_is_invariant_and_does_not_publish_completion() {
    let mut actual = CounterState::default();
    let admitted = actual
        .forward(Request { attempt: true, new_hold: 1, temporary_live: true })
        .expect("positive admission");
    actual.spent += 1;
    let before = actual;
    assert_eq!(actual.complete(admitted, true), Err(Fault::Invariant));
    assert_eq!(actual, before);
}
