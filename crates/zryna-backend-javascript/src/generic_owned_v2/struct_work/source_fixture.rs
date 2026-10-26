//! Existing exact source/DTO goes through every real authority before the test callback.
use zryna_ir::generic_v1::owned_v2 as owned;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{layouts::verify_layouts, owned_v4},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn with_wide(scopes: usize, inspect: impl FnOnce(&owned::VerifiedOwnedProgram<'_>)) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../zryna-native-mir/src/generic_owned_v2/struct_tests/fixtures");
    let source = std::fs::read_to_string(root.join(format!("wide-{scopes}.zry")))
        .expect("unchanged genuine source");
    let packet = std::fs::read(root.join(format!("wide-{scopes}.json")))
        .expect("unchanged frozen untrusted DTO");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("source");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("packet decoding"),
        &sources,
    )
    .expect("exact original source authentication");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations =
        resolve_declarations(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("all original bodies");
    let instances = owned_v4::discover(&bodies).expect("actual original proof and discovery");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("linux");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("ABI claims"),
        &linear,
        &linux,
    )
    .expect("runtime authority");
    let claim =
        owned_v4::produce_claim(&instances, &linear, &linux).expect("source-produced claims");
    let bytes = owned::wire::v4::encode(&claim).expect("wire");
    let program = owned::verify_v4(
        owned::wire::v4::decode(&bytes).expect("decode"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("genuine v4 seal FIRST");
    inspect(&program);
}
