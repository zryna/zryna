//! The negative original must stop before a context, claim, seal or backend artifact.
use super::{authority, source};
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    resolve_declarations,
};
use zryna_source::UntrustedSpan;
pub(super) fn check() {
    let (sources, syntax, _) = source::authenticate(authority::NEGATIVE);
    let entry = sources.verify_file_id(0).expect("main entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
    )
    .expect("original declarations");
    let expected = authority::case(authority::NEGATIVE);
    let offset = |name: &str| {
        u32::try_from(expected["diagnostic"][name].as_u64().expect("frozen span"))
            .expect("small byte offset")
    };
    let span = sources
        .verify_span(UntrustedSpan { file: 0, start: offset("start"), end: offset("end") })
        .expect("exact original Clone keyword");
    let BodyTypeFailure::Diagnostics(errors) = check_body_types(&declarations)
        .expect_err("unused opaque original cannot issue a discovery context")
    else {
        panic!("owning semantic source diagnostic");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M7002");
    assert_eq!(errors[0].primary_span(), Some(span));
    let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect_err("independent new original issuer rejects")
    else {
        panic!("owning IR source diagnostic");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M7002");
    assert_eq!(errors[0].primary_span(), Some(span));
}
