//! Fixed seeded control arithmetic; grants no source, count-state or execution authority.
use serde_json::Value;
use zryna_ownership_runtime_abi::generic_execution_accounting::{CounterState, Fault, Request};
fn state(value: &Value) -> CounterState {
    let values = value.as_array().expect("fixed A/L/S/H seed");
    assert_eq!(values.len(), 4);
    CounterState {
        attempts: values[0].as_u64().expect("A"),
        live: values[1].as_u64().expect("L"),
        spent: values[2].as_u64().expect("S"),
        held: values[3].as_u64().expect("H"),
    }
}
fn fault(value: &Value) -> Option<Fault> {
    match value.as_str() {
        None => None,
        Some("Denied") => Some(Fault::Denied),
        Some("Invariant") => Some(Fault::Invariant),
        _ => panic!("unknown frozen outcome"),
    }
}
fn event(actual: &mut CounterState, step: &Value) {
    let before = *actual;
    let expected_fault = fault(&step["fault"]);
    match step["kind"].as_str().expect("fixed event kind") {
        "forward" => {
            let request = &step["request"];
            let admission = actual.forward(Request {
                attempt: request["attempt"].as_bool().expect("actual attempt class"),
                new_hold: request["new_hold"].as_u64().expect("exact prospective debt"),
                temporary_live: request["temporary_live"].as_bool().expect("physical temporary"),
            });
            if let Some(expected_fault) = expected_fault {
                assert_eq!(admission.err(), Some(expected_fault));
                assert!(
                    step["canonical_status"].is_null(),
                    "no canonical status from global denial/invariant"
                );
            } else {
                let admission = admission.expect("independent admitted event");
                assert_eq!(*actual, state(&step["reserved"]), "prepaid before side effects");
                let success = step["success"].as_bool().expect("validated canonical result class");
                assert_eq!(
                    success,
                    step["canonical_status"] == 0,
                    "oracle status class only, not target evidence"
                );
                actual.complete(admission, success).expect("consume actual numeric reservation");
            }
        }
        "release" => {
            let positive = step["positive"].as_bool().expect("independently validated actual free");
            let result = actual.release(positive);
            assert_eq!(result, expected_fault.map_or(Ok(()), Err));
            assert_ne!(result, Err(Fault::Denied), "held cleanup cannot need forward quota");
        }
        _ => panic!("unknown frozen event"),
    }
    assert_eq!(*actual, state(&step["after"]), "exact S/A retention and H/L commit/refund");
    if expected_fault.is_some() {
        assert_eq!(*actual, before, "failed numeric step is atomic");
    }
}
#[test]
fn twenty_frozen_control_debt_and_counter_sequences_are_exact_atomic_and_repeatable() {
    use sha2::{Digest, Sha256};
    let bytes =
        include_bytes!("generic_execution_accounting_control/HELD_COUNTER_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "608f189fd8e21d44f567ee41ee3aedec6f65ccd1c8ddb35542bb8a8bbf20e9ce"
    );
    let oracle: Value = serde_json::from_slice(bytes).expect("pre-control-output fixed oracle");
    assert_eq!(oracle["limits"]["A"], zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS);
    assert_eq!(oracle["limits"]["L"], zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS);
    assert_eq!(oracle["limits"]["S"], zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS);
    let cases = oracle["cases"].as_array().expect("twenty finite independent rows");
    assert_eq!(cases.len(), 20);
    for case in cases {
        for _ in 0..2 {
            let mut actual = state(&case["pre"]);
            for step in case["events"].as_array().expect("exact event sequence") {
                event(&mut actual, step);
            }
        }
    }
}
