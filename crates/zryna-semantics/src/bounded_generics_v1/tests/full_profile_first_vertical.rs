//! Existing semantic support and current private execution gaps; no full-profile execution claim.
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    instantiation::owned_v5,
    resolve_declarations,
};
use zryna_ir::generic_v1::Failure;
use zryna_source::UntrustedSpan;
pub(super) mod source;

#[test]
fn six_complete_first_vertical_originals_authenticate_both_honest_provider_packets() {
    for case in source::POSITIVE.into_iter().chain([source::NEGATIVE]) {
        let (native_sources, native) = source::project(case, "native");
        let (ts_sources, typescript) = source::project(case, "typescript");
        assert!(native.is_bound_to(&native_sources));
        assert!(typescript.is_bound_to(&ts_sources));
        assert_eq!(native.files(), typescript.files(), "{case}: complete files");
    }
}

#[test]
fn five_full_contract_first_verticals_pass_existing_original_symbolic_body_typing() {
    for case in source::POSITIVE {
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            let declarations = resolve_declarations(
                SemanticInput::try_new(
                    &syntax,
                    &sources,
                    sources.verify_file_id(0).expect("module"),
                )
                .expect("original semantic input"),
            )
            .expect("all original declarations");
            check_body_types(&declarations)
                .expect("accepted full-contract original source capability");
        }
    }
}

#[test]
fn unused_opaque_clone_still_rejects_before_closed_discovery_at_the_frozen_keyword() {
    let expected = source::expected();
    let row = expected["cases"]
        .as_array()
        .expect("six cases")
        .iter()
        .find(|row| row["name"] == source::NEGATIVE)
        .expect("frozen opaque control");
    assert_eq!(row["no_partial_program"], true);
    assert_eq!(row["diagnostic"]["code"], "ZRYNA-M7002");
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(source::NEGATIVE, provider);
        assert_eq!(syntax.files()[0].functions[0].name.text, "forbidden");
        let declarations = resolve_declarations(
            SemanticInput::try_new(&syntax, &sources, sources.verify_file_id(0).expect("module"))
                .expect("original semantic input"),
        )
        .expect("all original declarations before any closed instance");
        let BodyTypeFailure::Diagnostics(errors) = check_body_types(&declarations)
            .expect_err("opaque original rejection prevents body/discovery/program authority")
        else {
            panic!("owning original semantic rejection")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7002");
        let number = |field: &str| {
            u32::try_from(row["diagnostic"][field].as_u64().expect("frozen offset"))
                .expect("small literal offset")
        };
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start: number("start"), end: number("end") })
            .expect("independent authenticated original keyword");
        assert_eq!(errors[0].primary_span(), Some(span));
        let text = sources.source(span.file()).expect("exact source").text();
        assert_eq!(
            &text[usize::try_from(span.start()).expect("offset")
                ..usize::try_from(span.end()).expect("offset")],
            "clone"
        );
    }
}

#[test]
fn current_struct_clone_private_v5_execution_gaps_are_baseline_observations_only() {
    // This test is NOT an accepted full-profile execution test. It names the restricted
    // private v5 gate and guards against accidentally reporting rejection as goal completion.
    for case in source::POSITIVE {
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            let declarations = resolve_declarations(
                SemanticInput::try_new(
                    &syntax,
                    &sources,
                    sources.verify_file_id(0).expect("module"),
                )
                .expect("original input"),
            )
            .expect("all original declarations");
            let bodies = check_body_types(&declarations).expect("valid full-contract source");
            let Failure::Diagnostics(errors) = owned_v5::discover(&bodies).expect_err(
                "current restricted private v5 does not grant these full-profile executions",
            ) else {
                panic!("source-owned gap observation, not infrastructure failure")
            };
            assert!(!errors.is_empty());
            println!("CURRENT PRIVATE V5 GAP ONLY {case}/{provider}: {errors:?}");
        }
    }
}
