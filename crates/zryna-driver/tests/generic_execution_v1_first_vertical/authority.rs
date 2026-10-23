//! Frozen source/runtime expectations, independent of compiler artifacts and observations.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub(super) const POSITIVE: [&str; 5] = [
    "scalar-i32-clone",
    "scalar-bool-clone",
    "nominal-enum-clone",
    "option-nominal-enum-clone",
    "fixed-array-identity",
];
pub(super) const NEGATIVE: &str = "opaque-clone-unused";
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-full-profile-first-vertical")
}
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn read(name: &str) -> Vec<u8> {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json"))
            .expect("independent original input identities");
    let identity = &identities["files"][name];
    let bytes = std::fs::read(directory().join(name)).expect("complete frozen original input");
    assert_eq!(
        u64::try_from(bytes.len()).expect("bounded bytes"),
        identity["bytes"].as_u64().expect("frozen byte count")
    );
    assert_eq!(digest(&bytes), identity["sha256"], "fixed original input {name}");
    bytes
}
pub(super) fn expected() -> serde_json::Value {
    let bytes = read("SOURCE_EXPECTED.json");
    assert_eq!(digest(&bytes), "99570cb50f7c75af51c88339584817f65069495e5f25eb6c04b78a724ffde16d");
    serde_json::from_slice(&bytes).expect("pre-provider fixed runtime/source oracles")
}
pub(super) fn case(name: &str) -> serde_json::Value {
    expected()["cases"]
        .as_array()
        .expect("six frozen source cases")
        .iter()
        .find(|row| row["name"] == name)
        .expect("exact frozen case")
        .clone()
}
pub(super) fn check_all() {
    let identities: serde_json::Value =
        serde_json::from_str(include_str!("fixture-identities.json")).expect("frozen input table");
    for name in identities["files"].as_object().expect("all input identities").keys() {
        let _ = read(name);
    }
    assert_eq!(expected()["cases"].as_array().expect("cases").len(), 6);
    assert_eq!(case(NEGATIVE)["semantic"], "reject");
    for name in POSITIVE {
        assert_eq!(case(name)["semantic"], "accept");
    }
}
