//! Exact existing literal/packet crosses all authorities before any budget-helper assertion.
use zryna_ir::generic_v1::owned_v2 as owned;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{layouts::verify_layouts, owned_v5},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn with_nested_prefix(inspect: impl FnOnce(&owned::VerifiedOwnedProgram<'_>)) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-struct-clone-v5");
    let source = std::fs::read_to_string(root.join("literals/nested-prefix.zry"))
        .expect("unchanged complete literal source");
    let packet = std::fs::read(root.join("nested-prefix-native.json"))
        .expect("existing frozen genuine provider packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("exact complete source map");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("closed original provider DTO"),
        &sources,
    )
    .expect("full original source authentication");
    let entry = sources.verify_file_id(0).expect("literal entry module");
    let declarations =
        resolve_declarations(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .expect("all original declarations");
    let bodies = check_body_types(&declarations).expect("all original body capabilities");
    let instances = owned_v5::discover(&bodies).expect("original ownership and v5 discovery");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("independent sealed linear layout");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("independent sealed Linux layout");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("ABI claims"),
        &linear,
        &linux,
    )
    .expect("independent runtime authority");
    let claim = owned_v5::produce_claim(&instances, &linear, &linux)
        .expect("genuine source-produced v5 claim");
    let bytes = owned::wire::v5::encode(&claim).expect("distinct bounded v5 wire");
    let program = owned::verify_v5(
        owned::wire::v5::decode(&bytes).expect("distinct opaque v5 carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("complete independent source/type/owner/runtime v5 seal FIRST");
    inspect(&program);
}
