//! Independent expectations, pinned to bytes frozen before target compilation.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-v4-nested")
}
pub(super) fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&read_fixture(name)).expect("frozen fixture JSON")
}
pub(super) fn read_fixture(name: &str) -> Vec<u8> {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json"))
            .expect("independently pinned fixture identities");
    let identity = &identities["files"][name];
    let bytes = std::fs::read(fixture_directory().join(name)).expect("frozen complete input");
    assert_eq!(Some(bytes.len() as u64), identity["bytes"].as_u64(), "{name}: byte count");
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), identity["sha256"], "{name}: frozen hash");
    bytes
}
pub(super) fn execution(case: &str) -> serde_json::Value {
    let all: serde_json::Value = serde_json::from_str(include_str!("execution-oracle.json"))
        .expect("independent pre-compilation execution oracle");
    let fixed = all["cases"][case].clone();
    assert!(fixed.is_object(), "case outside fixed oracle");
    fixed
}
pub(super) fn allocations(case: &str) -> usize {
    usize::try_from(execution(case)["allocations"].as_u64().expect("literal count"))
        .expect("bounded allocation count")
}
pub(super) fn events(case: &str, fault: usize) -> Vec<String> {
    let fixed = execution(case);
    let events = if fault == 0 {
        &fixed["success"]
    } else {
        &fixed["allocationFailures"][fault.to_string()]
    };
    events
        .as_array()
        .expect("fixed event list")
        .iter()
        .map(|event| event.as_str().expect("literal event").to_owned())
        .collect()
}
pub(super) fn check_frozen_authority() {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json"))
            .expect("independent fixture identity freeze");
    for name in identities["files"].as_object().expect("fixed identities").keys() {
        let _ = read_fixture(name);
    }
    let freeze = json("source-expected-freeze.json");
    assert_eq!(freeze["caseCount"], 13);
    assert_eq!(freeze["positiveCount"], 8);
    assert_eq!(freeze["negativeCount"], 5);
    for (case, allocations) in
        super::source::POSITIVE_CASES.into_iter().zip([0, 2, 1, 1, 0, 0, 1, 4])
    {
        let expected = json(&format!("{case}-expected.json"));
        let mut copied = execution(case);
        assert_eq!(
            copied.as_object_mut().expect("fixed case").remove("allocations"),
            Some(allocations.into())
        );
        assert_eq!(
            copied, expected,
            "execution oracle is an exact copy of independent frozen expectations"
        );
        assert_eq!(expected["decision"], "admit");
        assert_eq!(expected["result"], 7);
        assert_eq!(expected["retryAfterEveryFailure"], true);
        assert_eq!(expected["allocationFailures"].as_object().expect("faults").len(), allocations);
        for fault in 1..=allocations {
            assert!(expected["allocationFailures"][fault.to_string()].is_array());
        }
    }
}
