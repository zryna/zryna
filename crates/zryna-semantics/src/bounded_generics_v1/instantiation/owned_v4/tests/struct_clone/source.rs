//! Complete frozen originals and honest fresh provider identities, with no source derivatives.
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};

pub(super) const POSITIVE: [&str; 7] = [
    "nested-prefix",
    "repeat-source",
    "shared-owner",
    "phantom-opaque",
    "fixed-member",
    "family-fields",
    "copy-struct",
];
pub(super) const OPAQUE: [&str; 3] = ["opaque-direct", "opaque-stored", "opaque-inactive"];
pub(super) const OWNERSHIP: [&str; 2] = ["moved-source", "exclusive-source"];
pub(super) const PROVIDERS: [&str; 2] = ["native", "typescript"];
fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-clone-v5")
}
pub(super) fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(directory().join(name)).expect("frozen fixture"))
        .expect("fixture JSON")
}
pub(super) fn expected(case: &str) -> serde_json::Value {
    json(&format!("literals/{case}-expected.json"))
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let freeze_bytes = std::fs::read(directory().join("SOURCE_EXPECTED_FREEZE.json"))
        .expect("pre-provider source and expectation freeze");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze_bytes)),
        "46cea891843e537976c96a7cfc6fc00debd6c069fd5570865bcc42e5ad53fbe7"
    );
    let freeze: serde_json::Value =
        serde_json::from_slice(&freeze_bytes).expect("pre-provider freeze");
    let row = freeze["cases"]
        .as_array()
        .expect("twelve fixed originals")
        .iter()
        .find(|row| row["case"] == case)
        .expect("fixed literal");
    let text =
        std::fs::read(directory().join("literals").join(row["source"].as_str().expect("path")))
            .expect("complete literal bytes");
    assert_eq!(format!("{:x}", Sha256::digest(&text)), row["sourceSha256"]);
    let expected_bytes =
        std::fs::read(directory().join("literals").join(row["expected"].as_str().expect("path")))
            .expect("independent expected bytes");
    assert_eq!(format!("{:x}", Sha256::digest(&expected_bytes)), row["expectedSha256"]);
    let identities = json("packet-identities.json");
    let receipt = identities["records"]
        .as_array()
        .expect("actual processes")
        .iter()
        .find(|row| row["case"] == case && row["provider"] == provider)
        .expect("honest provider identity");
    let packet = std::fs::read(directory().join(receipt["packet"].as_str().expect("packet")))
        .expect("complete actual provider DTO");
    assert_eq!(format!("{:x}", Sha256::digest(&packet)), receipt["packetSha256"]);
    assert_eq!(receipt["exactSourceSha256"], row["sourceSha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("UTF8 literal"),
    }])
    .expect("exact source map, no appended wrapper");
    let syntax = verify_snapshot(decode_snapshot(&packet).expect("closed v5 raw schema"), &sources)
        .expect("complete original source authentication");
    (sources, syntax)
}
