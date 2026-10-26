//! Exact independent source-owner diagnostics; no generated claim can substitute proof.
use super::source;
use zryna_source::UntrustedSpan;
fn reject(case: &str) {
    let row = source::row(case);
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
fn double_field() {
    reject("double-field");
}
#[test]
fn whole_after_hole() {
    reject("whole-after-hole");
}
#[test]
fn write_borrowed_field() {
    reject("write-borrowed-field");
}
#[test]
fn unrestored_loop_field() {
    reject("unrestored-loop-field");
}
#[test]
fn wrong_field_rhs() {
    reject("wrong-field-rhs");
}
