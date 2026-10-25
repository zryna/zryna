//! Exact independent source-owner diagnostics; no generated claim can substitute proof.
use super::loan_source as source;
use zryna_source::UntrustedSpan;
fn reject(case: &str) {
    let expected = source::diagnostic_expected();
    let row = expected["cases"]
        .as_array()
        .expect("ten hostile frozen originals")
        .iter()
        .find(|r| r["case"] == case)
        .expect("exact hostile case");
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
fn exclusive_parent_frozen_by_child() {
    reject("parent-frozen");
}
#[test]
fn overlapping_exclusive_borrow() {
    reject("overlap-exclusive");
}
#[test]
fn move_while_borrowed() {
    reject("root-moved");
}
#[test]
fn affine_borrowed_read() {
    reject("affine-read");
}
#[test]
fn shared_replacement() {
    reject("shared-write");
}
#[test]
fn wrong_call_access() {
    reject("wrong-access");
}
#[test]
fn wrong_call_referent() {
    reject("wrong-referent");
}
#[test]
fn opaque_referent_clone() {
    reject("opaque-clone");
}
#[test]
fn escaping_loan_result() {
    reject("borrow-escape");
}
#[test]
fn duplicate_exclusive_call_alias() {
    reject("exclusive-call-alias");
}
