//! Syntax/source authentication for independently frozen Struct Clone proposal literals.
//! Neither semantic Clone admission nor a v5 program/runtime result is claimed here.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-clone-v5")
}
fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(directory().join(name)).expect("frozen fixture"))
        .expect("fixture JSON")
}
fn frozen_hash(bytes: &[u8], hash: &serde_json::Value) {
    assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash.as_str().expect("frozen hash"));
}

#[test]
fn twelve_exact_clone_literals_authenticate_actual_native_and_both_complete_frozen_packets() {
    let freeze_bytes = std::fs::read(directory().join("SOURCE_EXPECTED_FREEZE.json"))
        .expect("source and independent expectation freeze");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze_bytes)),
        "46cea891843e537976c96a7cfc6fc00debd6c069fd5570865bcc42e5ad53fbe7",
        "independent literal/oracle freeze predates every fresh provider"
    );
    let freeze: serde_json::Value =
        serde_json::from_slice(&freeze_bytes).expect("pre-provider freeze");
    assert_eq!(freeze["positiveCases"], 7);
    assert_eq!(freeze["negativeCases"], 5);
    let identities = json("packet-identities.json");
    let cases = freeze["cases"].as_array().expect("twelve frozen cases");
    assert_eq!(cases.len(), 12);
    assert_eq!(identities["records"].as_array().expect("fresh packets").len(), 24);
    for case in cases {
        let name = case["case"].as_str().expect("frozen case identity");
        let source = std::fs::read(
            directory().join("literals").join(case["source"].as_str().expect("source path")),
        )
        .expect("complete unchanged original literal");
        frozen_hash(&source, &case["sourceSha256"]);
        assert_eq!(
            u64::try_from(source.len()).expect("small literal"),
            case["sourceBytes"].as_u64().expect("source count")
        );
        let expected = std::fs::read(
            directory().join("literals").join(case["expected"].as_str().expect("expected path")),
        )
        .expect("independent original expected bytes");
        frozen_hash(&expected, &case["expectedSha256"]);
        let sources = SourceMap::build(vec![SourceFileInput {
            path: "main.zry".into(),
            text: String::from_utf8(source).expect("original UTF8"),
        }])
        .expect("exact original source map; no appended wrapper");
        let lexed = zryna_frontend::native_lexer::lex(&sources).expect("actual native lexer");
        let raw = zryna_frontend::native_parser::v5::parse_v5_candidate(&sources, &lexed)
            .expect("actual native v5 candidate");
        let actual =
            v5::verify_snapshot(raw, &sources).expect("actual native exact-source authentication");
        for provider in ["native", "typescript"] {
            let row = identities["records"]
                .as_array()
                .expect("named actual process packets")
                .iter()
                .find(|row| row["case"] == name && row["provider"] == provider)
                .expect("honest original provider identity");
            assert_eq!(row["acceptedSyntaxWithoutDiagnostics"], true);
            assert_eq!(row["exactSourceSha256"], case["sourceSha256"]);
            let packet =
                std::fs::read(directory().join(row["packet"].as_str().expect("named packet")))
                    .expect("complete fresh provider DTO");
            frozen_hash(&packet, &row["packetSha256"]);
            assert_eq!(
                u64::try_from(packet.len()).expect("bounded packet"),
                row["packetBytes"].as_u64().expect("packet count")
            );
            let syntax = v5::verify_snapshot(
                v5::decode_snapshot(&packet).expect("closed v5 packet"),
                &sources,
            )
            .expect("complete original source authentication");
            assert_eq!(actual.files(), syntax.files(), "{name}/{provider}: complete file equality");
            assert!(syntax.is_bound_to(&sources));
        }
    }
}
