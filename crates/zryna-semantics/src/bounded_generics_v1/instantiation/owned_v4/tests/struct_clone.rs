//! Frozen Clone capability/ownership behavior; no future v5 API or executable claim is invented.
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    instantiation::{discover as discover_types, layouts::verify_layouts, owned_v4},
    resolve_declarations,
};
use zryna_ir::generic_v1::{Failure, owned_v2 as owned};
use zryna_layout::StorageTarget;
use zryna_source::UntrustedSpan;
mod source;

#[test]
fn complete_provider_packets_authenticate_all_twelve_clone_originals() {
    for case in source::POSITIVE.into_iter().chain(source::OPAQUE).chain(source::OWNERSHIP) {
        let (native_sources, native) = source::project(case, "native");
        let (ts_sources, typescript) = source::project(case, "typescript");
        assert!(native.is_bound_to(&native_sources));
        assert!(typescript.is_bound_to(&ts_sources));
        assert_eq!(native.files(), typescript.files(), "{case}: complete original equality");
    }
}

#[test]
fn seven_original_clone_bodies_derive_capability_from_every_actual_stored_member() {
    for case in source::POSITIVE {
        assert_eq!(source::expected(case)["accept"], true);
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            let declarations = resolve_declarations(
                SemanticInput::try_new(
                    &syntax,
                    &sources,
                    sources.verify_file_id(0).expect("original module"),
                )
                .expect("source-bound input"),
            )
            .expect("all original declarations");
            check_body_types(&declarations)
                .expect("original Clone proof, no closed specialization assumption");
        }
    }
}

#[test]
fn opaque_clone_rejects_at_the_frozen_keyword_in_original_unused_bodies() {
    for case in source::OPAQUE {
        let expected = source::expected(case);
        assert_eq!(expected["accept"], false);
        assert_eq!(expected["diagnostic"], "ZRYNA-M7002");
        assert_eq!(expected["noInstanceContext"], true);
        assert_eq!(expected["originalUnusedFunctionMustStillReject"], true);
        for provider in source::PROVIDERS {
            let (sources, syntax) = source::project(case, provider);
            assert_eq!(syntax.files()[0].functions.len(), 2);
            assert_eq!(syntax.files()[0].functions[0].name.text, "unused");
            assert_eq!(syntax.files()[0].functions[1].name.text, "entry");
            let declarations = resolve_declarations(
                SemanticInput::try_new(
                    &syntax,
                    &sources,
                    sources.verify_file_id(0).expect("module"),
                )
                .expect("original input"),
            )
            .expect("all original declarations");
            let BodyTypeFailure::Diagnostics(errors) = check_body_types(&declarations).expect_err(
                "original rejection before any body/discovery/instance context escapes",
            ) else {
                panic!("owning original capability diagnostic")
            };
            assert_eq!(errors.len(), 1, "{case}/{provider}: {errors:?}");
            assert_eq!(errors[0].code(), "ZRYNA-M7002");
            let at = &expected["primarySpan"];
            let number = |field: &str| {
                u32::try_from(at[field].as_u64().expect("frozen byte range"))
                    .expect("small literal offset")
            };
            let span = sources
                .verify_span(UntrustedSpan {
                    file: number("file"),
                    start: number("start"),
                    end: number("end"),
                })
                .expect("independently frozen source-authenticated keyword span");
            assert_eq!(errors[0].primary_span(), Some(span));
            let text = sources.source(span.file()).expect("same source authority").text();
            assert_eq!(
                &text[usize::try_from(span.start()).expect("offset")
                    ..usize::try_from(span.end()).expect("offset")],
                "clone"
            );
            // Discovery is intentionally unreachable: the opaque original remains invalid
            // even though its only export returns the closed scalar7 and never calls it.
        }
    }
}

#[test]
fn moved_or_exclusively_loaned_clone_sources_reject_before_owning_discovery() {
    for case in source::OWNERSHIP {
        let expected = source::expected(case);
        assert_eq!(expected["diagnostic"], "ZRYNA-M7007");
        assert_eq!(expected["noInstanceContext"], true);
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
            let bodies = check_body_types(&declarations)
                .expect("Clone capability is separate from availability");
            let Failure::Diagnostics(errors) = owned_v4::discover(&bodies).expect_err(
                "owning source check precedes generated discovery and instance context",
            ) else {
                panic!("owning original source diagnostic")
            };
            assert_eq!(errors.len(), 1, "{case}/{provider}: {errors:?}");
            assert_eq!(errors[0].code(), "ZRYNA-M7007");
            let span = errors[0].primary_span().expect("original source use-site");
            sources.resolve(span).expect("primary span authenticates against the same SourceMap");
        }
    }
}

#[test]
fn old_v4_owning_source_and_claim_production_remain_closed_to_struct_clone() {
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
            .expect("original declarations");
            let bodies = check_body_types(&declarations).expect("valid source Clone capability");
            let Failure::Diagnostics(errors) = owned::check_original_bodies_v4(&syntax, &sources)
                .expect_err("v4 source issuer retains its exact String/family Clone domain")
            else {
                panic!("v4 owning source rejection")
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-I7001");
            let Failure::Diagnostics(errors) = owned_v4::discover(&bodies)
                .expect_err("no v4 ownership/instance context for Struct Clone")
            else {
                panic!("closed semantic v4 issuer")
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M3008");
            // Type discovery/layout seals alone provide no Clone source/owned authority.
            let instances = discover_types(&bodies).expect("closed type inventory only");
            let linear =
                verify_layouts(&instances, StorageTarget::Linear32V1).expect("linear shape only");
            let linux =
                verify_layouts(&instances, StorageTarget::LinuxX8664V1).expect("Linux shape only");
            let keys = instances.function_keys().collect::<Vec<_>>();
            assert!(
                owned::produce_claim_v4(&syntax, &sources, &linear, &linux, &keys).is_err(),
                "closed layout or Copy/zero-drop shape cannot acquire v4 Struct Clone authority"
            );
        }
    }
}
