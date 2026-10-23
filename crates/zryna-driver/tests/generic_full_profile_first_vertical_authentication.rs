//! Complete frozen first-vertical source authentication; no private/full execution grant.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-full-profile-first-vertical")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(directory().join(name)).expect("complete frozen input")
}
fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&read(name)).expect("fixture JSON")
}

#[test]
fn actual_native_and_twelve_complete_frozen_packets_authenticate_six_unchanged_literals() {
    let freeze = read("SOURCE_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze)),
        "05d459c1cf8acbae0fa016d6aa6ff6ed2a91ee898d65f705b43a3225a3d20c8f"
    );
    let freeze: serde_json::Value = serde_json::from_slice(&freeze).expect("pre-provider freeze");
    for row in freeze["files"].as_array().expect("six originals plus independent expectation") {
        let bytes = read(row["path"].as_str().expect("frozen path"));
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
        assert_eq!(
            u64::try_from(bytes.len()).expect("bounded input"),
            row["bytes"].as_u64().expect("frozen byte count")
        );
    }
    let expected = json("SOURCE_EXPECTED.json");
    let identities = json("packet-identities.json");
    let cases = expected["cases"].as_array().expect("six independently frozen cases");
    assert_eq!(cases.len(), 6);
    assert_eq!(identities["records"].as_array().expect("twelve named actual processes").len(), 12);
    for case in cases {
        let name = case["name"].as_str().expect("frozen source identity");
        let source = read(case["source"].as_str().expect("complete source path"));
        assert_eq!(format!("{:x}", Sha256::digest(&source)), case["source_sha256"]);
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: String::from_utf8(source).expect("exact original UTF8"),
        }])
        .expect("exact source map");
        let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual native lexer");
        let raw = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
            .expect("actual native v5 parser");
        let actual = v5::verify_snapshot(raw, &sources)
            .expect("actual native complete-source authentication");
        for provider in ["native", "typescript"] {
            let row = identities["records"]
                .as_array()
                .expect("actual packets")
                .iter()
                .find(|row| row["case"] == name && row["provider"] == provider)
                .expect("honest provider label");
            assert_eq!(row["acceptedSyntaxWithoutDiagnostics"], true);
            assert_eq!(row["exactSourceSha256"], case["source_sha256"]);
            let packet = read(row["packet"].as_str().expect("named actual packet"));
            assert_eq!(format!("{:x}", Sha256::digest(&packet)), row["packetSha256"]);
            let frozen = v5::verify_snapshot(
                v5::decode_snapshot(&packet).expect("strict raw v5 DTO"),
                &sources,
            )
            .expect("frozen complete original source authentication");
            assert!(frozen.is_bound_to(&sources));
            assert_eq!(actual.files(), frozen.files(), "{name}/{provider}: complete source facts");
        }
    }
}
