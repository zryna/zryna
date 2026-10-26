//! Frozen seeded arithmetic/effect controls; no public seal or historical source-count claim.
use sha2::{Digest, Sha256};
use zryna_ownership_runtime_abi::generic_execution_accounting::{CounterState, Fault, Request};
fn state(row: &serde_json::Value) -> CounterState {
    CounterState {
        attempts: row["A"].as_u64().expect("A"),
        live: row["L"].as_u64().expect("L"),
        spent: row["S"].as_u64().expect("S"),
        held: row["H"].as_u64().expect("H"),
    }
}
fn acquire(actual: &mut CounterState, positive: bool, success: bool) -> Result<(), Fault> {
    let admitted =
        actual.forward(Request { attempt: positive, new_hold: 1, temporary_live: positive })?;
    actual.complete(admitted, success)
}
fn reserve(
    actual: &mut CounterState,
    positive: bool,
    old_positive: bool,
    success: bool,
) -> Result<(), Fault> {
    let admitted =
        actual.forward(Request { attempt: positive, new_hold: 0, temporary_live: positive })?;
    if positive {
        actual.complete_growth(admitted, success, old_positive)
    } else {
        actual.complete(admitted, success)
    }
}
fn event(id: &str, actual: &mut CounterState) -> Result<(), Fault> {
    match id {
        "empty-allocate-exact-held-credit"
        | "empty-allocate-prospective-credit-extra"
        | "forged-overflow-context" => acquire(actual, false, true),
        "positive-allocate-attempt-exact" | "positive-allocate-attempt-extra" => {
            acquire(actual, true, true)
        }
        "within-capacity-at-attempt-ceiling" | "within-capacity-status-extra" => {
            reserve(actual, false, true, true)
        }
        "first-growth-from-empty-exact" => reserve(actual, true, false, true),
        "replacement-growth-exact" | "positive-reserve-attempt-extra" => {
            reserve(actual, true, true, true)
        }
        "reserve-admitted-target-allocation-failure" | "prepared-RHS-survives-reserve-failure" => {
            reserve(actual, true, true, false)
        }
        "canonical-size-capacity-only-forward" => reserve(actual, false, true, false),
        "positive-storage-release-at-total-ceiling" | "release-without-storage-debt" => {
            actual.release(true)
        }
        "empty-storage-release-spends-credit" => actual.release(false),
        "clone-prefix-second-string-failure" => {
            acquire(actual, true, true)?; // New Vec storage debt once.
            acquire(actual, true, true)?; // First copied String debt once.
            acquire(actual, true, false) // Failed second String keeps A/S, cancels only its H.
        }
        "zero-reserve-noop" => reserve(actual, false, false, true),
        _ => panic!("every independent frozen row needs an explicit classified event"),
    }
}
#[test]
fn eighteen_frozen_vec_cases_preserve_debt_transfer_failure_and_cleanup_arithmetic() {
    let bytes =
        include_bytes!("generic_execution_accounting_vec/HELD_COUNTER_EXPECTED_FREEZE.json");

    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "5c6cbfc840dd06b29a409b5994002c294bf7436dd7d6be7a0abddede76f632f0"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("unchanged pre-output oracle");
    let cases = oracle["cases"].as_array().expect("eighteen independent cases");
    assert_eq!(cases.len(), 18);
    for row in cases {
        let id = row["id"].as_str().expect("fixed event identifier");
        let mut actual = state(&row["pre"]);
        let expected = &row["expected"];
        let kind = expected["admission"].as_str().expect("outcome");
        let outcome = if kind.starts_with("ADMITTED") {
            Ok(())
        } else if kind.starts_with("DENIED") {
            Err(Fault::Denied)
        } else {
            Err(Fault::Invariant)
        };
        assert_eq!(event(id, &mut actual), outcome, "{id}: admission class");
        assert_eq!(actual, state(&expected["post"]), "{id}: exact atomic poststate");
        if !expected["after_cleanup"].is_null() {
            let count = if id == "clone-prefix-second-string-failure" {
                5
            } else if id == "prepared-RHS-survives-reserve-failure" {
                3
            } else {
                1
            };
            for _ in 0..count {
                actual
                    .release(actual.live != 0)
                    .expect("mandatory held cleanup has no quota denial");
            }
            assert_eq!(actual, state(&expected["after_cleanup"]), "{id}: exact held cleanup");
        }
    }
}
#[test]
fn growth_completion_rejects_wrong_reservation_or_missing_storage_authority_atomically() {
    for (mut actual, request, old_positive) in [
        (
            CounterState { attempts: 1, live: 1, spent: 1, held: 1 },
            Request { attempt: true, new_hold: 1, temporary_live: true },
            true,
        ),
        (
            CounterState { attempts: 1, live: 1, spent: 1, held: 1 },
            Request { attempt: false, new_hold: 0, temporary_live: false },
            true,
        ),
        (
            CounterState { attempts: 1, live: 0, spent: 1, held: 0 },
            Request { attempt: true, new_hold: 0, temporary_live: true },
            false,
        ),
        (
            CounterState { attempts: 1, live: 0, spent: 1, held: 1 },
            Request { attempt: true, new_hold: 0, temporary_live: true },
            true,
        ),
    ] {
        let admitted = actual
            .forward(request)
            .expect("valid arithmetic reservation before typed growth completion");
        let reserved = actual;
        assert_eq!(actual.complete_growth(admitted, true, old_positive), Err(Fault::Invariant));
        assert_eq!(actual, reserved, "rejected completion publishes no live/debt state");
    }
}
#[test]
fn failed_growth_retains_attempt_then_retry_retires_old_backing_without_extra_event_or_debt() {
    let mut actual = CounterState { attempts: 1, live: 1, spent: 1, held: 1 };
    reserve(&mut actual, true, true, false).expect("admitted allocator failure");
    assert_eq!(actual, CounterState { attempts: 2, live: 1, spent: 2, held: 1 });
    reserve(&mut actual, true, true, true).expect("successful retry");
    assert_eq!(actual, CounterState { attempts: 3, live: 1, spent: 3, held: 1 });
    actual.release(true).expect("existing storage debt");
    assert_eq!(actual, CounterState { attempts: 3, live: 0, spent: 4, held: 0 });
}
#[test]
fn interleaved_growth_admission_cannot_publish_another_completion() {
    let mut actual = CounterState { attempts: 1, live: 1, spent: 1, held: 1 };
    let admitted = actual
        .forward(Request { attempt: true, new_hold: 0, temporary_live: true })
        .expect("growth");
    actual.spent += 1;
    let before = actual;
    assert_eq!(actual.complete_growth(admitted, true, true), Err(Fault::Invariant));
    assert_eq!(actual, before);
}
