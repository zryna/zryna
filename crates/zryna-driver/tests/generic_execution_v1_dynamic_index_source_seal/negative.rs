//! Exact independent source-owner diagnostics; no generated claim can substitute proof.
use super::source;
use zryna_source::UntrustedSpan;
fn reject(case: &str) {
    let row = source::diagnostic(case);
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        let offset = |name: &str| {
            u32::try_from(
                row["primary"][name].as_u64().expect("pre-implementation UTF8 byte offset"),
            )
            .expect("bounded original offset")
        };
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start: offset("start"), end: offset("end") })
            .expect("exact source-owned primary range");
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
                .expect_err("original misuse cannot issue source authority")
        else {
            panic!("owning exact source diagnostic, not unfinished/internal rejection");
        };
        assert_eq!(errors.len(), 1, "{case}/{provider}");
        assert_eq!(
            errors[0].code(),
            row["code"].as_str().expect("frozen code"),
            "{case}/{provider}"
        );
        assert_eq!(errors[0].primary_span(), Some(span), "{case}/{provider}");
        let entry = sources.verify_file_id(0).expect("actual original entry");
        let declarations = zryna_semantics::bounded_generics_v1::resolve_declarations(
            zryna_semantics::bounded_generics_v1::SemanticInput::try_new(&syntax, &sources, entry)
                .expect("same authenticated declarations"),
        )
        .expect("hostile bodies retain valid declarations");
        let zryna_semantics::bounded_generics_v1::body_types::BodyTypeFailure::Diagnostics(errors) =
            zryna_semantics::bounded_generics_v1::body_types::check_execution_body_types(
                &declarations,
            )
            .expect_err("the explicit execution semantic API preserves original rejection")
        else {
            panic!("exact semantic source diagnostic, not an infrastructure rejection");
        };
        assert_eq!(errors.len(), 1, "semantic {case}/{provider}");
        assert_eq!(
            errors[0].code(),
            row["code"].as_str().expect("frozen code"),
            "semantic {case}/{provider}"
        );
        assert_eq!(errors[0].primary_span(), Some(span), "semantic {case}/{provider}");
    }
}
#[test]
fn dynamic_array_overlap() {
    reject("dynamic-array-overlap");
}
#[test]
fn vector_constant_overlap() {
    reject("vector-constant-overlap");
}
#[test]
fn shared_element_write() {
    reject("shared-element-write");
}
#[test]
fn owner_move_while_indexed() {
    reject("owner-move-while-indexed");
}
#[test]
fn opaque_indexed_consume() {
    reject("opaque-indexed-consume");
}
#[test]
fn indexed_rhs_moves_owner() {
    reject("indexed-rhs-moves-owner");
}
