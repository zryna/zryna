//! Exact source permission controls precede every target/runtime authority.
use super::{continuations, source};
fn rejection(
    sources: &zryna_source::SourceMap,
    syntax: &zryna_syntax::v5::VerifiedProjectSyntaxV5,
    row: &serde_json::Value,
) {
    let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
        zryna_ir::generic_v1::execution_v1::check_original_bodies(syntax, sources)
            .expect_err("source permission must fail before native authority")
    else {
        panic!("owning source diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), row["code"].as_str().expect("frozen diagnostic"));
    let span = sources
        .verify_span(zryna_source::UntrustedSpan {
            file: 0,
            start: u32::try_from(row["primary"]["start"].as_u64().expect("start"))
                .expect("bounded start"),
            end: u32::try_from(row["primary"]["end"].as_u64().expect("end")).expect("bounded end"),
        })
        .expect("source-owned primary span");
    assert_eq!(errors[0].primary_span(), Some(span));
}
#[test]
fn indexed_source_hostiles_still_fail_before_native_permission() {
    for case in [
        "dynamic-array-overlap",
        "vector-constant-overlap",
        "shared-element-write",
        "owner-move-while-indexed",
        "opaque-indexed-consume",
        "indexed-rhs-moves-owner",
    ] {
        let expected = source::diagnostic(case);
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            rejection(&sources, &syntax, &expected);
        }
    }
    let case = "indexed-push-rhs-relocates-owner";
    let expected = continuations::diagnostic(case);
    for provider in continuations::PROVIDERS {
        let (sources, syntax) = continuations::project(case, provider);
        rejection(&sources, &syntax, &expected);
    }
}
