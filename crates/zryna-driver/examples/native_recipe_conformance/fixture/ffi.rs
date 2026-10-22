//! Genuine syntax -> semantics -> IR -> MIR -> object issuers; no host compiler.
use super::{DECLARATIONS, HEADER, POLICY, SOURCE, ZRYNA_PATH};
use zryna_driver::NativeRecipeRequirements;
use zryna_semantics::native_c_v0::{
    LibraryMaterial,
    body::{compose_private_boundaries, verify_bodies},
};
use zryna_source::SourceFileInput;
use zryna_source::SourceMap;
pub(super) fn capture() -> (SourceMap, NativeRecipeRequirements) {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: ZRYNA_PATH.into(), text: SOURCE.into() }])
            .expect("original source map");
    let syntax = zryna_syntax::native_c_source_v0::authenticate_sources(&sources)
        .expect("independent source authentication");
    let declarations = zryna_semantics::native_c_v0::verify(
        DECLARATIONS,
        &sources,
        &syntax,
        &[LibraryMaterial {
            library_id: "fixture-sqlite-c-v0@0",
            header_bytes: HEADER,
            policy_bytes: POLICY,
        }],
        zryna_syntax::native_c_v0::TARGET,
    )
    .expect("original source-bound declarations");
    let bodies = verify_bodies(&sources, &declarations).expect("independent semantic body replay");
    let authority =
        compose_private_boundaries(&sources, &bodies).expect("original layout/ownership issuer");
    let ir = zryna_native_c_ir::lower(&sources, &authority).expect("independent extension IR");
    let machine = zryna_native_mir::native_c_v0::lower(&ir).expect("independent machine admission");
    let symbol = machine
        .functions()
        .find(|function| function.name() == "copied")
        .expect("exact entry")
        .entry()
        .symbol
        .clone();
    let object = zryna_backend_native::native_c_v0::resources::emit_byte_entries(
        &machine,
        &[&symbol],
        zryna_backend_native::select_object_target(zryna_backend_native::NATIVE_OBJECT_TARGET)
            .expect("exact target"),
    )
    .expect("audited original object");
    let requirements =
        zryna_driver::native_c_v0::resource_identity::handle_link_requirements(&object)
            .expect("original requirements");
    (sources, requirements)
}
