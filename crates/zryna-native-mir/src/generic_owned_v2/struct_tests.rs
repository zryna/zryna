//! Fixed source/independent layout expectations; no target observation supplies an oracle.
use super::{StorageTarget, lower};
use zryna_ir::generic_v1::owned_v2 as owned;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{layouts::verify_layouts, owned_v4},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};

mod amplification;
mod future_nested;

#[test]
fn authentic_struct_construction_and_copy_field_reach_native_mir() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-v4");
    let source = std::fs::read_to_string(root.join("construction.zry")).expect("frozen source");
    let bytes = std::fs::read(root.join("construction-reference.json")).expect("frozen DTO");
    with_source(source, &bytes, |program| {
        assert!(
            program.functions().iter().enumerate().any(|(index, function)| {
                function.blocks.iter().flat_map(|block| &block.instructions).any(|instruction| {
                    matches!(
                        program.extension(index, instruction.result.id),
                        Some(owned::raw::Operation::CopyStructField { .. })
                    )
                })
            }),
            "the accepted box.value source must exercise the actual new projection"
        );
        let mir = lower(program).expect("Struct field fanout and Copy projection are precharged");
        assert_eq!(mir.functions().len(), 1);
        assert_eq!(mir.functions()[0].result(), 1, "fixed source returns i32 seven");
        let structure = program
            .layouts(StorageTarget::LinuxX8664V1)
            .types()
            .find(|ty| ty.key().first() == Some(&0x12))
            .expect("Box<i32>");
        assert_eq!(
            mir.width(zryna_ir::generic_v1::raw::Type::Stored(structure.id().index()))
                .expect("private representation"),
            1
        );
    });
}

fn with_source(
    source: String,
    bytes: &[u8],
    inspect: impl FnOnce(&owned::VerifiedOwnedProgram<'_>),
) {
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
        .expect("source authority");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(bytes).expect("closed raw DTO"),
        &sources,
    )
    .expect("complete original syntax authentication");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original semantic authority"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("all original opaque bodies");
    let instances = owned_v4::discover(&bodies).expect("actual v4 source demand");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("raw runtime"),
        &linear,
        &linux,
    )
    .expect("independent runtime issuer");
    let claim = owned_v4::produce_claim(&instances, &linear, &linux).expect("untrusted v4 claims");
    assert!(claim.extensions.iter().flatten().any(|extension| matches!(
        extension.operation,
        owned::raw::Operation::StructConstruct { .. }
    )));

    let program = owned::verify_v4(
        owned::wire::v4::decode(&owned::wire::v4::encode(&claim).expect("wire")).expect("decode"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent original/type/ownership v4 seal");
    inspect(&program);
}
