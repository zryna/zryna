//! Whole-issuer denial controls preserve exact original diagnostics and original source brands.
use super::{execution, fixture, runtime};
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::{BodyTypeFailure, check_body_types},
    resolve_declarations,
};
use zryna_ir::generic_v1::{Failure, execution_v1::raw};
use zryna_source::UntrustedSpan;
#[test]
fn opaque_vec_clone_and_opaque_root_downgrade_keep_exact_original_m7002() {
    for case in ["opaque-vec-clone-unused", "opaque-downgrade-unused"] {
        let expected = fixture::row(case);
        let span =
            expected["expected"]["span_utf8"].as_array().expect("pre-provider keyword range");
        let offset = |n: usize| {
            u32::try_from(span[n].as_u64().expect("frozen offset")).expect("bounded offset")
        };
        for provider in fixture::PROVIDERS {
            let (sources, syntax) = fixture::project(case, provider);
            let entry = sources.verify_file_id(0).expect("actual entry");
            let keyword = sources
                .verify_span(UntrustedSpan { file: 0, start: offset(0), end: offset(1) })
                .expect("original keyword");
            let declarations = resolve_declarations(
                SemanticInput::try_new(&syntax, &sources, entry).expect("actual source input"),
            )
            .expect("original declarations");
            let BodyTypeFailure::Diagnostics(errors) = check_body_types(&declarations)
                .expect_err("no body or instance context from opaque original")
            else {
                panic!("owning original diagnostics")
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M7002");
            assert_eq!(errors[0].primary_span(), Some(keyword));
            let Failure::Diagnostics(errors) = execution::check_original_bodies(&syntax, &sources)
                .expect_err("independent original check before replay")
            else {
                panic!("source owning diagnostic")
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M7002");
            assert_eq!(errors[0].primary_span(), Some(keyword));
        }
    }
}
#[test]
fn whole_issuer_rejects_changed_container_identity_or_pending_cleanup_and_retries() {
    for case in ["vec-owned-prepare", "shared-weak-count"] {
        fixture::with_claim(case, "native", |sources, syntax, linear, linux, claim| {
            let runtime = runtime::verify_v1(
                runtime::raw_v1(linear, linux).expect("actual declarations"),
                linear,
                linux,
            )
            .expect("runtime owner");
            let issue = |claim: &raw::Program| {
                execution::verify(
                    execution::wire::decode(
                        &execution::wire::encode(claim).expect("bounded claims"),
                    )
                    .expect("new carrier"),
                    syntax,
                    sources,
                    sources.verify_file_id(0).expect("entry"),
                    linear,
                    linux,
                    &runtime,
                )
            };
            let mut changed = claim.clone();
            let instruction = changed
                .functions
                .iter_mut()
                .flat_map(|function| &mut function.blocks)
                .flat_map(|block| &mut block.instructions)
                .find(|instruction| {
                    matches!(
                        instruction.operation,
                        raw::Operation::VecConstruct { .. }
                            | raw::Operation::SharedConstruct { .. }
                    )
                })
                .expect("genuine source preparation");
            let string = linear
                .types()
                .find(|ty| ty.key() == [2])
                .expect("actual original String identity")
                .id()
                .index();
            match &mut instruction.operation {
                raw::Operation::VecConstruct { ty, .. }
                | raw::Operation::SharedConstruct { ty, .. } => *ty = string,
                _ => unreachable!("selected actual preparation"),
            }
            assert!(issue(&changed).is_err(), "whole source issuer denies changed exact type");
            let mut changed = claim.clone();
            let function = changed
                .functions
                .iter_mut()
                .find(|function| function.executable_export.is_some())
                .expect("actual entry");
            let failure = function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .find_map(|instruction| match instruction.operation {
                    raw::Operation::VecConstruct { failure, .. }
                    | raw::Operation::SharedConstruct { failure, .. } => Some(failure),
                    _ => None,
                })
                .expect("actual pending allocation failure");
            function.cleanup_plans[usize::try_from(failure).expect("bounded plan")]
                .actions
                .pop()
                .expect("prepared source owner");
            assert!(issue(&changed).is_err(), "omitted prepared-owner cleanup never seals");
            issue(claim).expect("pristine whole new seal after both rejections");
        });
    }
}
#[test]
fn same_bytes_foreign_sources_and_runtime_layout_brands_cannot_issue_a_container_seal() {
    fixture::with_claim("shared-weak-count", "native", |sources, syntax, linear, linux, claim| {
        fixture::with_claim(
            "shared-weak-count",
            "typescript",
            |foreign, foreign_syntax, foreign_linear, foreign_linux, _| {
                assert_eq!(syntax.files(), foreign_syntax.files());
                assert_ne!(sources.identity(), foreign.identity());
                assert_eq!(
                    linear.types().map(zryna_layout::generic_v1::TypeView::key).collect::<Vec<_>>(),
                    foreign_linear
                        .types()
                        .map(zryna_layout::generic_v1::TypeView::key)
                        .collect::<Vec<_>>()
                );
                let runtime = runtime::verify_v1(
                    runtime::raw_v1(linear, linux).expect("actual declarations"),
                    linear,
                    linux,
                )
                .expect("runtime owner");
                let foreign_runtime = runtime::verify_v1(
                    runtime::raw_v1(foreign_linear, foreign_linux)
                        .expect("foreign genuine declarations"),
                    foreign_linear,
                    foreign_linux,
                )
                .expect("independent foreign runtime");
                let bytes = execution::wire::encode(claim).expect("genuine claim bytes");
                let decoded = || execution::wire::decode(&bytes).expect("new carrier");
                assert!(
                    execution::verify(
                        decoded(),
                        foreign_syntax,
                        foreign,
                        foreign.verify_file_id(0).expect("foreign entry"),
                        linear,
                        linux,
                        &runtime
                    )
                    .is_err()
                );
                assert!(
                    execution::verify(
                        decoded(),
                        syntax,
                        sources,
                        sources.verify_file_id(0).expect("entry"),
                        foreign_linear,
                        foreign_linux,
                        &foreign_runtime
                    )
                    .is_err()
                );
                assert!(
                    execution::verify(
                        decoded(),
                        syntax,
                        sources,
                        sources.verify_file_id(0).expect("entry"),
                        linear,
                        linux,
                        &foreign_runtime
                    )
                    .is_err()
                );
                execution::verify(
                    decoded(),
                    syntax,
                    sources,
                    sources.verify_file_id(0).expect("entry"),
                    linear,
                    linux,
                    &runtime,
                )
                .expect("pristine whole source/layout/runtime retry");
            },
        );
    });
}
