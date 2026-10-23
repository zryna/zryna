//! Frozen fixture loader; provider equality never substitutes for source authentication.
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::PathBuf;
use zryna_ir::generic_v1::{
    Failure,
    owned_v2::raw::{Operation, Program},
};
use zryna_layout::generic_v1::VerifiedLayouts;
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-v4-nested")
}
fn json(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(directory().join(name)).expect("frozen fixture"))
        .expect("fixture JSON")
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let freeze = json("source-expected-freeze.json");
    let row = freeze["cases"]
        .as_array()
        .expect("fixed cases")
        .iter()
        .find(|row| row["id"] == case)
        .expect("independent literal");
    let text = std::fs::read(directory().join(row["source"].as_str().expect("path")))
        .expect("complete original bytes");
    assert_eq!(format!("{:x}", Sha256::digest(&text)), row["sourceSha256"]);
    let receipts = json("packet-identities.json");
    let receipt = receipts["records"]
        .as_array()
        .expect("actual process records")
        .iter()
        .find(|row| row["case"] == case && row["provider"] == provider)
        .expect("actual process identity");
    let packet = std::fs::read(directory().join(receipt["packet"].as_str().expect("packet")))
        .expect("complete untrusted provider DTO");
    assert_eq!(format!("{:x}", Sha256::digest(&packet)), receipt["packetSha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("UTF8 source"),
    }])
    .expect("independent complete source map");
    let syntax = verify_snapshot(decode_snapshot(&packet).expect("closed raw schema"), &sources)
        .expect("full source authentication before semantic proof");
    (sources, syntax)
}
pub(super) fn reject(case: &str, provider: &str) -> Vec<zryna_diagnostics::Diagnostic> {
    use crate::bounded_generics_v1::{
        SemanticInput, body_types::check_body_types, resolve_declarations,
    };
    let (sources, syntax) = project(case, provider);
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, sources.verify_file_id(0).expect("entry"))
            .expect("bound input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("all opaque original bodies first");
    let Failure::Diagnostics(errors) = super::super::super::discover(&bodies)
        .expect_err("original ownership must reject before issuing a v4 context")
    else {
        panic!("structured original rejection")
    };
    errors
}
pub(super) fn check_keys(case: &str, linear: &VerifiedLayouts, linux: &VerifiedLayouts) {
    let oracle = json("key-oracle.json");
    let Some(expected) = oracle["cases"][case].as_object() else {
        return;
    };
    for layouts in [linear, linux] {
        let keys = layouts
            .types()
            .map(|ty| {
                ty.key().iter().fold(String::new(), |mut encoded, byte| {
                    write!(&mut encoded, "{byte:02x}").expect("String formatting");
                    encoded
                })
            })
            .collect::<Vec<_>>();
        for (role, key) in expected {
            assert!(
                keys.iter().any(|actual| actual == key.as_str().expect("literal hex")),
                "{case}/{role}: independent original identity and ordered arguments"
            );
        }
    }
}
pub(super) fn check_prefix_claim(claim: &Program) {
    let expected = json("prefix-plan-oracle.json");
    let operations = claim.extensions[0]
        .iter()
        .map(|extension| match &extension.operation {
            Operation::StringLiteral(bytes) => serde_json::json!({"result":extension.result,
            "kind":"StringLiteral","bytes":String::from_utf8(bytes.clone()).expect("UTF8")} ),
            Operation::CloneString(value) => serde_json::json!({"result":extension.result,
            "kind":"CloneString","value":value}),
            Operation::StructConstruct { fields, .. } => {
                serde_json::json!({"result":extension.result,
            "kind":"StructConstruct","fields":fields})
            }
            other => panic!("unplanned original operation: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(operations), expected["extensions"]);
    let steps = claim.plans[0]
        .steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "block":step.block,"position":step.position,"end_loans":step.end_loans,
                "cleanup":step.cleanup,"failure":step.failure,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(steps), expected["steps"]);
}
