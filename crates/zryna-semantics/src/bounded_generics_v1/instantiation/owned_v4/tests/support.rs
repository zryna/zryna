//! No frontend dependency: both raw provider packets independently authenticate complete source.
use super::super::{discover, produce_claim};
use crate::bounded_generics_v1::{
    SemanticInput, body_types::check_body_types, instantiation::layouts::verify_layouts,
    resolve_declarations,
};
use std::path::PathBuf;
use zryna_ir::generic_v1::{Failure, owned_v2 as owned};
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{
    RawProjectSyntaxSnapshot, VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot,
};

pub(super) const PROVIDERS: [&str; 2] = ["native", "typescript"];
pub(super) fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-struct-v4-semantics")
}
pub(super) fn source(case: &str) -> String {
    let sources: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory().join("sources.json")).expect("literal complete originals"),
    )
    .expect("source inventory JSON");
    sources[case].as_str().expect("literal source case").to_owned()
}
pub(super) fn candidate(case: &str, provider: &str) -> RawProjectSyntaxSnapshot {
    decode_snapshot(
        &std::fs::read(directory().join(format!("{case}-{provider}.json")))
            .expect("actual untrusted provider candidate"),
    )
    .expect("closed v5 raw schema")
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source(case) }])
            .expect("exact independent source authority");
    let syntax = verify_snapshot(candidate(case, provider), &sources)
        .expect("complete source authentication before any semantics");
    (sources, syntax)
}

pub(super) fn with_claim(
    case: &str,
    provider: &str,
    test: impl FnOnce(
        owned::raw::Program,
        &VerifiedProjectSyntaxV5,
        &SourceMap,
        &VerifiedLayouts,
        &VerifiedLayouts,
    ),
) {
    let (sources, syntax) = project(case, provider);
    with_project(&sources, &syntax, test);
}

pub(super) fn with_project(
    sources: &SourceMap,
    syntax: &VerifiedProjectSyntaxV5,
    test: impl FnOnce(
        owned::raw::Program,
        &VerifiedProjectSyntaxV5,
        &SourceMap,
        &VerifiedLayouts,
        &VerifiedLayouts,
    ),
) {
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(syntax, sources, entry).expect("source-bound semantic input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("all original opaque bodies typed first");
    let context = discover(&bodies).expect("v4 original ownership before complete discovery");
    let linear =
        verify_layouts(context.instances(), StorageTarget::Linear32V1).expect("linear layout");
    let linux =
        verify_layouts(context.instances(), StorageTarget::LinuxX8664V1).expect("Linux layout");
    let claim = produce_claim(&context, &linear, &linux).expect("source-owned untrusted v4 claim");
    seal(&claim, syntax, sources, &linear, &linux).expect("independent complete v4 issuer");
    test(claim, syntax, sources, &linear, &linux);
}

pub(super) fn seal(
    claim: &owned::raw::Program,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
) -> Result<(), Failure> {
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux).expect("runtime claims"),
        linear,
        linux,
    )
    .expect("independent runtime authority");
    owned::verify_v4(
        owned::wire::v4::decode(&owned::wire::v4::encode(claim)?)?,
        syntax,
        sources,
        sources.verify_file_id(0).expect("entry"),
        linear,
        linux,
        &runtime,
    )
    .map(|_| ())
}

pub(super) fn original_rejection(case: &str, provider: &str) -> Vec<zryna_diagnostics::Diagnostic> {
    let (sources, syntax) = project(case, provider);
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, sources.verify_file_id(0).expect("entry"))
            .expect("source input"),
    )
    .expect("original declarations");
    let bodies =
        check_body_types(&declarations).expect("rejection follows genuine original typing");
    let Failure::Diagnostics(errors) =
        discover(&bodies).expect_err("original ownership must reject")
    else {
        panic!("structured owning diagnostic required")
    };
    errors
}
