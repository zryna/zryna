//! Original nominal payload facts, not argument spelling, govern concrete Copy transport.
use super::tests::{claim, with_claim};
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, resolve_declarations,
    tests::body_fixtures::project,
};
use zryna_ir::generic_v1::{Failure, owned_v2, raw};

mod acceptance;
mod negative;
mod packet;

const BOX: &str = "interface Box<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;value:T;}";
const MARKER: &str =
    "interface Marker<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;code:bool;}";

fn original_types(files: &[(&str, &str)]) {
    let p = project(files);
    let entry = p.sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&p.syntax, &p.sources, entry).expect("source authority"),
    )
    .expect("original declarations");
    check_body_types(&declarations).expect("original opaque body types before Copy classification");
}

fn seal(
    claim: &owned_v2::raw::Program,
    syntax: &zryna_syntax::v5::VerifiedProjectSyntaxV5,
    sources: &zryna_source::SourceMap,
    linear: &zryna_layout::generic_v1::VerifiedLayouts,
    linux: &zryna_layout::generic_v1::VerifiedLayouts,
) {
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux).expect("runtime claims"),
        linear,
        linux,
    )
    .expect("independent runtime ABI");
    owned_v2::verify(
        owned_v2::wire::decode(&owned_v2::wire::encode(claim).expect("wire")).expect("decode"),
        syntax,
        sources,
        sources.verify_file_id(0).expect("entry"),
        linear,
        linux,
        &runtime,
    )
    .expect("source replay, exact nominal identity, typed phi and cleanup authority");
}
