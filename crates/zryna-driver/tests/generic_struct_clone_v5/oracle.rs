//! Exact pre-provider oracles remain independent of compiler claims, layouts and runtime.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-clone-v5")
}
pub(super) fn read_fixture(name: &str) -> Vec<u8> {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json"))
            .expect("independent fixture identity freeze");
    let identity = &identities["files"][name];
    let bytes = std::fs::read(directory().join(name)).expect("complete frozen input");
    assert_eq!(
        u64::try_from(bytes.len()).expect("bounded input"),
        identity["bytes"].as_u64().expect("frozen byte count")
    );
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), identity["sha256"], "{name}");
    bytes
}
pub(super) fn execution(case: &str) -> serde_json::Value {
    let all: serde_json::Value = serde_json::from_str(include_str!("execution-oracle.json"))
        .expect("fixed independent original runtime expectations");
    let value = all["cases"][case].clone();
    assert!(value.is_object(), "case outside fixed oracle");
    value
}
pub(super) fn allocations(case: &str) -> usize {
    execution(case)["allocationAttempts"].as_array().expect("frozen attempts").len()
}
pub(super) fn clone_source(case: &str, attempt: usize) -> usize {
    let fixed = execution(case);
    let attempts = fixed["allocationAttempts"].as_array().expect("fixed attempts");
    let row = &attempts[attempt - 1];
    if row["operation"] == "literal" {
        return 0;
    }
    let role = row["role"].as_str().expect("frozen allocation role");
    let (owner, path) = role.split_once('.').expect("frozen whole-source leaf role");
    let source_owner = match owner {
        "copied" | "first" | "second" => "source",
        "activeCopy" => "active",
        _ => panic!("clone role outside the exact frozen literals"),
    };
    let source_role = format!("{source_owner}.{path}");
    let original = attempts
        .iter()
        .find(|row| row["role"] == source_role)
        .expect("frozen original source allocation role");
    assert_eq!(original["operation"], "literal");
    usize::try_from(original["attempt"].as_u64().expect("source attempt"))
        .expect("bounded source identity")
}
pub(super) fn operation(case: &str, attempt: usize) -> String {
    execution(case)["allocationAttempts"][attempt - 1]["operation"]
        .as_str()
        .expect("literal or clone")
        .to_owned()
}
pub(super) fn events(case: &str, fault: usize) -> Vec<String> {
    let expected = execution(case);
    let attempts = expected["allocationAttempts"].as_array().expect("fixed attempts");
    assert!(fault <= attempts.len());
    let successful = if fault == 0 { attempts.len() } else { fault - 1 };
    let mut events = attempts[..successful]
        .iter()
        .map(|attempt| {
            format!(
                "alloc:{}:{}",
                attempt["attempt"].as_u64().expect("attempt"),
                attempt["bytes"].as_str().expect("literal bytes")
            )
        })
        .collect::<Vec<_>>();
    let releases = if fault == 0 {
        &expected["successReleaseAttemptIds"]
    } else {
        events.push(format!("fail:{fault}"));
        &expected["faults"][fault - 1]["releaseAttemptIds"]
    };
    for release in releases.as_array().expect("fixed exact release order") {
        let id =
            usize::try_from(release.as_u64().expect("release identity")).expect("small identity");
        events.push(format!("drop:{id}:{}", attempts[id - 1]["bytes"].as_str().expect("bytes")));
    }
    events
}
pub(super) fn check_frozen_authority() {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json"))
            .expect("fixed original input hashes");
    for name in identities["files"].as_object().expect("input identities").keys() {
        let _ = read_fixture(name);
    }
    let freeze: serde_json::Value =
        serde_json::from_slice(&read_fixture("SOURCE_EXPECTED_FREEZE.json"))
            .expect("original pre-provider freeze");
    assert_eq!(freeze["positiveCases"], 7);
    assert_eq!(freeze["negativeCases"], 5);
    assert_eq!(freeze["futureThreeTargetsObservations"], 147);
    assert_eq!(freeze["futureNativeSigillCases"], 21);
    let mut total = 0;
    for (case, count) in super::source::POSITIVE_CASES.into_iter().zip([7, 6, 4, 0, 2, 2, 0]) {
        check_case(case, count);
        total += count;
    }
    assert_eq!(total, 21);
}

fn check_case(case: &str, count: usize) {
    let original: serde_json::Value =
        serde_json::from_slice(&read_fixture(&format!("literals/{case}-expected.json")))
            .expect("independent unchanged oracle");
    assert_eq!(execution(case), original);
    assert_eq!(original["accept"], true);
    assert_eq!(original["result"], 7);
    assert_eq!(original["structCarrierAllocations"], 0);
    assert_eq!(allocations(case), count);
    assert_eq!(original["faults"].as_array().expect("faults").len(), count);
    for (index, attempt) in
        original["allocationAttempts"].as_array().expect("attempts").iter().enumerate()
    {
        assert_eq!(attempt["attempt"], index + 1);
        assert!(matches!(attempt["operation"].as_str(), Some("literal" | "clone")));
        let source = clone_source(case, index + 1);
        if attempt["operation"] == "clone" {
            assert!(source > 0 && source <= index);
            assert_eq!(attempt["bytes"], original["allocationAttempts"][source - 1]["bytes"]);
        } else {
            assert_eq!(source, 0);
        }
    }
    for (index, fault) in original["faults"].as_array().expect("faults").iter().enumerate() {
        assert_eq!(fault["failAttempt"], index + 1);
        assert_eq!(fault["returnsScalar"], false);
        assert_eq!(fault["retryResult"], 7);
        assert_eq!(fault["retryReleaseAttemptIds"], original["successReleaseAttemptIds"]);
    }
}
