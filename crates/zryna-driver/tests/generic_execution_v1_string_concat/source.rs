//! Actual fresh packets, semantic discovery, and independent new execution issuer.
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::{self, raw};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1 as semantic_execution, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};
pub(crate) fn with_claim(case: &str, provider: &str, inspect: impl FnOnce(&raw::Program)) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-string-concat");
    let text = std::fs::read(base.join("literals").join(format!("{case}.zry")))
        .expect("pre-provider frozen original source");
    let oracle: serde_json::Value = serde_json::from_slice(
        &std::fs::read(base.join("SOURCE_EXPECTED_FREEZE.json"))
            .expect("pre-provider original oracle"),
    )
    .expect("exact source oracle");
    let expected = oracle["cases"]
        .as_array()
        .expect("frozen source cases")
        .iter()
        .find(|r| r["case"] == case)
        .expect("unchanged additive case");
    assert_eq!(format!("{:x}", Sha256::digest(&text)), expected["sha256"]);
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(text).expect("original UTF8"),
    }])
    .expect("actual original identity");
    let packet = std::fs::read(base.join(format!("{case}-{provider}.json")))
        .expect("actual fresh named provider packet");
    let syntax =
        verify_snapshot(decode_snapshot(&packet).expect("strict full provider DTO"), &sources)
            .expect("exact syntax/source authentication");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
    )
    .expect("actual original declarations");
    let bodies = check_execution_body_types(&declarations).expect("opaque original bodies");
    let instances = semantic_execution::discover(&bodies).expect("original ownership permission");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("same linear layout");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("same Linux layout");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("runtime declarations"),
        &linear,
        &linux,
    )
    .expect("same-universe runtime");
    let claim = semantic_execution::produce_claim(&instances, &linear, &linux)
        .expect("actual untrusted original replay");
    inspect(&claim);
    let bytes = execution_v1::wire::encode(&claim).expect("new execution carrier");
    let program = execution_v1::verify(
        execution_v1::wire::decode(&bytes).expect("new carrier decode"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("actual complete independent new issuer");
    let mir = zryna_native_mir::generic_execution_v1::lower(&program)
        .expect("genuine StringConcat native MIR");
    let target =
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("existing native target");
    let object = zryna_backend_native::generic_execution_v1::emit_object(&mir, target)
        .expect("genuine StringConcat native object");
    zryna_backend_native::generic_execution_v1::validate_object_inventory(object.bytes(), &mir)
        .expect("independent IR-derived concat/failure relocation inventory");
}
