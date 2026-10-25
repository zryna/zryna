//! Raw provider equality and input integrity never substitute for original compiler authority.
use super::support::{PROVIDERS, candidate, directory, project, source, with_claim};
use sha2::{Digest, Sha256};

#[test]
fn actual_native_and_typescript_packets_authenticate_identical_complete_originals() {
    let receipts: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory().join("provider-receipts.json"))
            .expect("actual emission receipt"),
    )
    .expect("receipt JSON");
    let records = receipts["records"].as_array().expect("bounded fixture records");
    assert_eq!(records.len(), 24);
    for case in [
        "construction",
        "fault-declaration-order",
        "fault-reordered-spelling",
        "fixed-copy",
        "moved-copy",
        "use-after-move",
        "shared-copy",
        "exclusive-copy",
        "opaque-projection",
        "opaque-identity",
        "nested-unused",
        "plain-enum",
    ] {
        assert_eq!(
            candidate(case, "native"),
            candidate(case, "typescript"),
            "{case}: full raw syntax parity"
        );
        for provider in PROVIDERS {
            let row = records
                .iter()
                .find(|row| row["case"] == case && row["provider"] == provider)
                .expect("exact emitted provider row");
            let bytes = std::fs::read(directory().join(format!("{case}-{provider}.json")))
                .expect("untrusted packet");
            assert_eq!(bytes.len() as u64, row["bytes"].as_u64().expect("recorded bytes"));
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                row["sha256"].as_str().expect("input SHA")
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(source(case).as_bytes())),
                row["sourceSha256"].as_str().expect("source SHA")
            );
            let (sources, syntax) = project(case, provider);
            assert!(
                syntax.is_bound_to(&sources),
                "every packet independently authenticates full source"
            );
        }
    }
}

#[test]
fn accepted_box_source_seals_through_semantic_v4_for_each_actual_provider() {
    for provider in PROVIDERS {
        with_claim("construction", provider, |claim, _, _, _, _| {
            assert_eq!(claim.extensions[0].len(), 2);
            assert_eq!(claim.plans[0].steps.len(), 1);
            assert!(claim.plans[0].steps[0].cleanup.is_empty());
        });
    }
}
