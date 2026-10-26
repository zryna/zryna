//! Existing frozen real source packets enter the independent complete execution issuer.
use std::path::Path;
use zryna_ir::generic_v1::execution_v1 as ir;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
pub(super) fn freeze() -> serde_json::Value {
    let freeze = serde_json::from_str(include_str!("BEFORE_OUTPUT_FREEZE.json"))
        .expect("pre-output expectations");
    super::originals::check(&freeze);
    freeze
}
pub(super) fn pins() -> serde_json::Value {
    serde_json::from_str(include_str!("INPUT_PINS.json")).expect("original packet/material pins")
}
fn read(row: &serde_json::Value) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(row["path"].as_str().expect("path"));
    let bytes = std::fs::read(path).expect("unchanged original material");
    assert_eq!(super::emit::digest(&bytes), row["sha256"]);
    if let Some(count) = row.get("bytes") {
        assert_eq!(
            u64::try_from(bytes.len()).expect("bounded material"),
            count.as_u64().expect("frozen byte count")
        );
    }
    bytes
}
pub(super) fn with_program(
    family: &str,
    provider: &str,
    inspect: impl FnOnce(&ir::VerifiedExecution<'_>, &[u8]),
) {
    let pins = pins();
    let row = &pins["cases"][family];
    check_census(row, provider);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(read(&row["source"])).expect("original UTF8"),
    }])
    .expect("original source owner");
    let packet = read(&row["packets"][provider]);
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("genuine strict original DTO"),
        &sources,
    )
    .expect("source-authenticated syntax");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original semantic input"),
    )
    .expect("original declarations");
    let bodies = check_execution_body_types(&declarations).expect("all original bodies");
    let instances = execution_v1::discover(&bodies).expect("genuine source demand");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("native");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("same-source runtime authority");
    let claim = execution_v1::produce_claim(&instances, &linear, &linux).expect("original replay");
    let wire = ir::wire::encode(&claim).expect("unchanged domain carrier");
    let program = ir::verify(
        ir::wire::decode(&wire).expect("strict carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent whole execution owner");
    inspect(&program, &wire);
}

fn check_census(row: &serde_json::Value, provider: &str) {
    let bytes = read(&row["packet_census"]);
    let census: serde_json::Value =
        serde_json::from_slice(&bytes).expect("original provider census");
    let case = Path::new(row["source"]["path"].as_str().expect("source path"))
        .file_stem()
        .expect("original case")
        .to_str()
        .expect("ASCII case");
    let record = census["records"]
        .as_array()
        .expect("actual records")
        .iter()
        .find(|r| r["case"] == case && r["provider"] == provider)
        .expect("original actual provider record");
    assert_eq!(record["exactSourceSha256"], row["source"]["sha256"]);
    assert_eq!(record["packetSha256"], row["packets"][provider]["sha256"]);
    assert_eq!(record["packetBytes"], row["packets"][provider]["bytes"]);
    assert_eq!(record["acceptedSyntaxWithoutDiagnostics"], true);
}
