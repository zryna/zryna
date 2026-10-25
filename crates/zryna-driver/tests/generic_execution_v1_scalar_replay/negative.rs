//! Exact semantic opaque-bound diagnostic survives closed i32 callers.
use super::source;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_execution_body_types},
    resolve_declarations,
};
use zryna_source::UntrustedSpan;
pub(super) fn check(case: &str) {
    let row = source::row(case);
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        let entry = sources.verify_file_id(0).expect("authenticated source entry");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, entry).expect("exact original syntax input"),
        )
        .expect("valid declarations around hostile body");
        let BodyTypeFailure::Diagnostics(errors) = check_execution_body_types(&declarations)
            .expect_err(
                "declared opaque bound cannot acquire scalar operations after closed substitution",
            )
        else {
            panic!("owning semantic diagnostic, not resource/internal failure");
        };
        let span = sources
            .verify_span(UntrustedSpan {
                file: 0,
                start: u32::try_from(row["primary"]["start"].as_u64().expect("frozen start"))
                    .expect("source range"),
                end: u32::try_from(row["primary"]["end"].as_u64().expect("frozen end"))
                    .expect("source range"),
            })
            .expect("exact source-owned operator");
        assert_eq!(errors.len(), 1, "{case}/{provider}");
        assert_eq!(errors[0].code(), row["code"].as_str().expect("pre-output exact M7002"));
        assert_eq!(errors[0].primary_span(), Some(span));
    }
}
