//! Frozen first-vertical literals authenticate before existing semantic/issuer observations.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};
pub(super) const POSITIVE: [&str; 5] = [
    "scalar-i32-clone",
    "scalar-bool-clone",
    "nominal-enum-clone",
    "option-nominal-enum-clone",
    "fixed-array-identity",
];
pub(super) const NEGATIVE: &str = "opaque-clone-unused";
pub(super) const PROVIDERS: [&str; 2] = ["native", "typescript"];
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-full-profile-first-vertical")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(directory().join(name)).expect("frozen complete input")
}
pub(super) fn expected() -> serde_json::Value {
    let bytes = read("SOURCE_EXPECTED.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "99570cb50f7c75af51c88339584817f65069495e5f25eb6c04b78a724ffde16d"
    );
    serde_json::from_slice(&bytes).expect("independent pre-provider expectations")
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let freeze = read("SOURCE_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze)),
        "05d459c1cf8acbae0fa016d6aa6ff6ed2a91ee898d65f705b43a3225a3d20c8f"
    );
    let expected = expected();
    let row = expected["cases"]
        .as_array()
        .expect("six frozen originals")
        .iter()
        .find(|row| row["name"] == case)
        .expect("frozen case");
    let text = read(row["source"].as_str().expect("complete source path"));
    assert_eq!(format!("{:x}", Sha256::digest(&text)), row["source_sha256"]);
    let identities: serde_json::Value = serde_json::from_slice(&read("packet-identities.json"))
        .expect("fresh named packet identities");
    let packet_row = identities["records"]
        .as_array()
        .expect("twelve actual process packets")
        .iter()
        .find(|row| row["case"] == case && row["provider"] == provider)
        .expect("honest actual provider identity");
    assert_eq!(packet_row["exactSourceSha256"], row["source_sha256"]);
    assert_eq!(packet_row["acceptedSyntaxWithoutDiagnostics"], true);
    let packet = read(packet_row["packet"].as_str().expect("named packet"));
    assert_eq!(format!("{:x}", Sha256::digest(&packet)), packet_row["packetSha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("complete original UTF8"),
    }])
    .expect("exact original source map");
    let syntax = verify_snapshot(decode_snapshot(&packet).expect("strict closed v5 DTO"), &sources)
        .expect("complete source authentication");
    (sources, syntax)
}
