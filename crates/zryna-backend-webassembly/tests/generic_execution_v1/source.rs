//! Real original packets and owning semantic/layout/runtime/IR issuers; no fabricated seals.
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{self, VerifiedExecution};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1 as semantic, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;
pub fn program<T>(case: &str, provider: &str, run: impl FnOnce(&VerifiedExecution<'_>) -> T) -> T {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-full-profile-first-vertical");
    let freeze = std::fs::read(base.join("SOURCE_FREEZE.json"))
        .expect("fixed genuine source/target control");
    assert_eq!(
        format!("{:x}", Sha256::digest(&freeze)),
        "05d459c1cf8acbae0fa016d6aa6ff6ed2a91ee898d65f705b43a3225a3d20c8f"
    );
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(base.join("SOURCE_EXPECTED.json"))
            .expect("fixed genuine source/target control"),
    )
    .expect("fixed genuine source/target control");
    let row = expected["cases"]
        .as_array()
        .expect("fixed genuine source/target control")
        .iter()
        .find(|row| row["name"] == case)
        .expect("fixed genuine source/target control");
    let text = std::fs::read(
        base.join(row["source"].as_str().expect("fixed genuine source/target control")),
    )
    .expect("fixed genuine source/target control");
    assert_eq!(format!("{:x}", Sha256::digest(&text)), row["source_sha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("fixed genuine source/target control"),
    }])
    .expect("fixed genuine source/target control");
    let packet = std::fs::read(base.join(format!("{case}-{provider}.json")))
        .expect("fixed genuine source/target control");
    let syntax = v5::verify_snapshot(
        v5::decode_snapshot(&packet).expect("fixed genuine source/target control"),
        &sources,
    )
    .expect("fixed genuine source/target control");
    let entry = sources.verify_file_id(0).expect("fixed genuine source/target control");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry)
            .expect("fixed genuine source/target control"),
    )
    .expect("fixed genuine source/target control");
    let bodies = check_body_types(&declarations).expect("genuine accepted original capabilities");
    let instances = semantic::discover(&bodies).expect("fixed genuine source/target control");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("fixed genuine source/target control");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("fixed genuine source/target control");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("fixed genuine source/target control"),
        &linear,
        &linux,
    )
    .expect("fixed genuine source/target control");
    let claim = semantic::produce_claim(&instances, &linear, &linux)
        .expect("fixed genuine source/target control");
    let bytes = execution_v1::wire::encode(&claim).expect("fixed genuine source/target control");
    let program = execution_v1::verify(
        execution_v1::wire::decode(&bytes).expect("fixed genuine source/target control"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("fixed genuine source/target control");
    run(&program)
}
