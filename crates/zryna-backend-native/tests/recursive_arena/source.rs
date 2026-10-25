//! Frozen actual packets, original semantic owners and the independent execution issuer.
use sha2::{Digest, Sha256};
use zryna_ir::generic_v1::execution_v1 as ir;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
fn pinned(row: &serde_json::Value, kind: &str) -> Vec<u8> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(base.join(row[kind].as_str().expect("fixed input path")))
        .expect("actual immutable provider input");
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row[format!("{kind}_sha256")]);
    assert_eq!(u64::try_from(bytes.len()).expect("input count"), row[format!("{kind}_bytes")]);
    bytes
}
pub(super) fn with_program(case: &str, check: impl FnOnce(&ir::VerifiedExecution<'_>)) {
    let rows: serde_json::Value = serde_json::from_str(include_str!("input-pins.json"))
        .expect("frozen preimplementation source identities");
    let row = &rows[case];
    let source = String::from_utf8(pinned(row, "source")).expect("source UTF8");
    let packet = pinned(row, "packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("actual original source");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("complete strict DTO"),
        &sources,
    )
    .expect("source syntax issuer");
    let entry = sources.verify_file_id(0).expect("main source");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"),
    )
    .expect("original declarations");
    let bodies =
        check_execution_body_types(&declarations).expect("all originals before generation");
    let instances = execution_v1::discover(&bodies).expect("genuine instance discovery");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let native =
        verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("native");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &native).expect("runtime claims"),
        &linear,
        &native,
    )
    .expect("same-domain runtime owner");
    let claim = execution_v1::produce_claim(&instances, &linear, &native).expect("source replay");
    let bytes = ir::wire::encode(&claim).expect("exact independent carrier");
    let program = ir::verify(
        ir::wire::decode(&bytes).expect("strict carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &native,
        &runtime,
    )
    .expect("independent whole execution owner");
    check(&program);
}
