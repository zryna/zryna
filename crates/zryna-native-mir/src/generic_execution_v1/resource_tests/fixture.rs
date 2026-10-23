//! Real frozen original syntax, semantics, independent layouts/runtime/wire/new issuer.
use sha2::{Digest, Sha256};
use zryna_ir::generic_v1::execution_v1 as execution;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
pub(super) const CASES: [&str; 5] = [
    "scalar-i32-clone",
    "scalar-bool-clone",
    "nominal-enum-clone",
    "option-nominal-enum-clone",
    "fixed-array-identity",
];
pub(super) const CONTAINERS: [&str; 4] =
    ["vec-copy", "vec-owned-prepare", "shared-weak-count", "opaque-count-unused"];
pub(super) const NEXT: [&str; 3] = ["vec-push", "weak-live", "weak-expired"];
pub(super) fn with_program(case: &str, inspect: impl FnOnce(&execution::VerifiedExecution<'_>)) {
    let container = CONTAINERS.contains(&case) || NEXT.contains(&case);
    assert!(container || CASES.contains(&case), "only frozen settled controls");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if container {
        "../../tests/m7-generic-execution-containers"
    } else {
        "../../tests/m7-generic-full-profile-first-vertical"
    });
    let source = std::fs::read_to_string(root.join(format!("literals/{case}.zry")))
        .expect("frozen exact original");
    let expected = std::fs::read(root.join(if container {
        "SOURCE_EXPECTED_FREEZE.json"
    } else {
        "SOURCE_EXPECTED.json"
    }))
    .expect("frozen independent expectations");
    assert_eq!(
        format!("{:x}", Sha256::digest(&expected)),
        if container {
            "d7d6d51db3eb9b51ed729b32aac8590a05b3102056ee2f88bc9c8baa51530d33"
        } else {
            "99570cb50f7c75af51c88339584817f65069495e5f25eb6c04b78a724ffde16d"
        }
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&expected).expect("independent source identities");
    let row = expected["cases"]
        .as_array()
        .expect("frozen cases")
        .iter()
        .find(|row| row[if container { "case" } else { "name" }] == case)
        .expect("fixed settled source");
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        row[if container { "sha256" } else { "source_sha256" }]
    );
    let packet = std::fs::read(root.join(format!("{case}-native.json")))
        .expect("complete frozen provider packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("complete source map");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("strict complete provider DTO"),
        &sources,
    )
    .expect("actual syntax issuer");
    let entry = sources.verify_file_id(0).expect("main entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("all original bodies");
    let instances =
        execution_v1::discover(&bodies).expect("new original-before-generated discovery");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("independent Linear layout");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("independent Linux layout");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("independent runtime issuer");
    let claim =
        execution_v1::produce_claim(&instances, &linear, &linux).expect("genuine source claims");
    let bytes = execution::wire::encode(&claim).expect("new wire");
    let program = execution::verify(
        execution::wire::decode(&bytes).expect("new decoded authority"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("NEW source/type/state/plan/layout/runtime seal");
    inspect(&program);
}
