//! Exact pre-provider sources and complete packets from two real named providers.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_source::{FileId, NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};
fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-execution-entry-one")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(directory().join(name)).expect("complete immutable entry1 input")
}
fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&read(name)).expect("frozen JSON")
}
pub(super) fn sources() -> SourceMap {
    let freeze = read("SOURCE_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze)),
        "ce0ed3cfefd521324026576b23a159a6af2e10371bca7851d597298bfabb64d5"
    );
    let freeze: serde_json::Value = serde_json::from_slice(&freeze).expect("pre-provider freeze");
    for row in freeze["files"].as_array().expect("two sources and oracle") {
        let bytes = read(row["path"].as_str().expect("exact frozen path"));
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
        assert_eq!(
            u64::try_from(bytes.len()).expect("bounded input"),
            row["bytes"].as_u64().expect("exact size")
        );
    }
    let expected = json("EXPECTED.json");
    let inputs = expected["sources"]
        .as_array()
        .expect("two sorted source paths")
        .iter()
        .rev()
        .map(|row| SourceFileInput {
            path: row["path"].as_str().expect("original path").into(),
            text: String::from_utf8(read(row["literal"].as_str().expect("complete source")))
                .expect("original UTF8"),
        })
        .collect();
    SourceMap::build(inputs).expect("actual source map sorts a=0/main=1")
}
pub(super) fn project(provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let sources = sources();
    let identities = json("packet-identities.json");
    let row = identities["records"]
        .as_array()
        .expect("two actual processes")
        .iter()
        .find(|row| row["provider"] == provider)
        .expect("honest provider name");
    assert_eq!(row["acceptedSyntaxWithoutDiagnostics"], true);
    let expected = json("EXPECTED.json");
    for original in expected["sources"].as_array().expect("complete two-file source") {
        assert!(
            row["exactSources"]
                .as_array()
                .expect("bound source hashes")
                .iter()
                .any(|source| source["path"] == original["path"]
                    && source["sha256"] == original["sha256"])
        );
    }
    let packet = read(row["packet"].as_str().expect("named process packet"));
    assert_eq!(format!("{:x}", Sha256::digest(&packet)), row["packetSha256"]);
    let syntax = verify_snapshot(decode_snapshot(&packet).expect("strict complete DTO"), &sources)
        .expect("genuine complete-source authentication");
    (sources, syntax)
}

pub(super) fn entry(sources: &SourceMap) -> FileId {
    sources
        .file_id(&NormalizedSourcePath::new("main.zry").expect("path"))
        .expect("genuine entry FileId")
}
