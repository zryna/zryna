//! Exact unchanged deep sources, real native syntax and frozen actual TypeScript packet authority.
use super::retain;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zryna_frontend::{native_lexer::lex, native_parser::v5::parse_v5_candidate};
use zryna_ir::generic_v1::execution_v1;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

fn base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-execution-recursive-owner")
}
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn input(name: &str) -> Vec<u8> {
    let identities =
        std::fs::read(base().join("input-identities.json")).expect("unchanged input census");
    assert_eq!(
        digest(&identities),
        "60adc69cb4984505c52cf14a865f8565c0645274762a8a509b6c6b9f625ccb5d"
    );
    let freeze: serde_json::Value =
        serde_json::from_slice(&identities).expect("original source freeze");
    let bytes = std::fs::read(base().join(name)).expect("unchanged original source input");
    let row = freeze["files"]
        .as_array()
        .expect("frozen inputs")
        .iter()
        .find(|row| row["path"] == name)
        .expect("named original input");
    assert_eq!(digest(&bytes), row["sha256"]);
    assert_eq!(u64::try_from(bytes.len()).expect("bounded input"), row["bytes"]);
    bytes
}

pub(super) fn actual(case: &str) {
    let text = String::from_utf8(input(&format!("literals/{case}.zry"))).expect("original UTF8");
    assert_eq!(
        digest(&input("SOURCE_EXPECTED_FREEZE.json")),
        "eb01f8297eb221c683d264fa35ab4f046f1a3ca3f7b561637090d77fd9b6ee24"
    );
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("original source map");
    let native = parse_v5_candidate(&sources, &lex(&sources).expect("original lexer"))
        .expect("native syntax candidate");
    let observation_bytes = std::fs::read(base().join("provider-observations.json"))
        .expect("unchanged provider observations");
    assert_eq!(
        digest(&observation_bytes),
        "278bada9cd8f9ad02e6241973ad81b2463065d823a5c176a3d4d729467b3d51d"
    );
    let observations: serde_json::Value =
        serde_json::from_slice(&observation_bytes).expect("provider observations");
    let row = observations["records"]
        .as_array()
        .expect("actual process records")
        .iter()
        .find(|row| row["case"] == case)
        .expect("same source process");
    let packet = std::fs::read(base().join(row["packet"].as_str().expect("packet path")))
        .expect("actual TS output");
    assert_eq!(digest(&packet), row["packet_sha256"]);
    assert_eq!(
        serde_json::to_value(&native).expect("native DTO"),
        serde_json::from_slice::<serde_json::Value>(&packet).expect("TS DTO")
    );
    for (provider, candidate) in
        [("native", native), ("typescript", decode_snapshot(&packet).expect("strict TS decode"))]
    {
        let syntax = verify_snapshot(candidate, &sources).expect("original source syntax owner");
        let entry = sources.verify_file_id(0).expect("entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
        )
        .expect("original declarations");
        let bodies = check_execution_body_types(&declarations).expect("original body types");
        let instances = semantic::discover(&bodies).expect("original semantic instances");
        let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
            .expect("original linear layouts");
        let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
            .expect("original native layouts");
        let runtime = runtime::verify_v1(
            runtime::raw_v1(&linear, &linux).expect("runtime candidate"),
            &linear,
            &linux,
        )
        .expect("same-universe runtime");
        let claim = semantic::produce_claim(&instances, &linear, &linux)
            .expect("original source execution claim");
        let wire = execution_v1::wire::encode(&claim).expect("new execution carrier");
        let program = execution_v1::verify(
            execution_v1::wire::decode(&wire).expect("execution carrier"),
            &syntax,
            &sources,
            entry,
            &linear,
            &linux,
            &runtime,
        )
        .expect("independent complete source/execution/cleanup owner");
        retain(case, provider, &program, &wire);
    }
}
