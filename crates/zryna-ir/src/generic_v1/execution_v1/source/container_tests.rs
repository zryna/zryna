//! Genuine named packets exercise source-only phases, never fabricated `SourceProof`.
use super::{Failure, UntrustedSpan, check_original_bodies, produce_claim, raw, verify_source};
mod fixture;
use fixture::{claims, keys, project};
const SETTLED: [&str; 4] =
    ["vec-copy", "vec-owned-prepare", "shared-weak-count", "opaque-count-unused"];
#[test]
fn all_nine_frozen_sources_authenticate_complete_named_packets() {
    for case in [
        "vec-copy",
        "vec-owned-prepare",
        "shared-weak-count",
        "opaque-count-unused",
        "opaque-vec-clone-unused",
        "opaque-downgrade-unused",
        "vec-push",
        "weak-live",
        "weak-expired",
    ] {
        let (a_sources, a) = project(case, "native");
        let (b_sources, b) = project(case, "typescript");
        assert!(a.is_bound_to(&a_sources));
        assert!(b.is_bound_to(&b_sources));
        assert_eq!(a.files(), b.files(), "{case}: entire source DTO, not a projection");
    }
}
#[test]
fn settled_container_originals_check_before_layouts_or_generated_instances() {
    for case in SETTLED {
        for provider in ["native", "typescript"] {
            let (sources, syntax) = project(case, provider);
            check_original_bodies(&syntax, &sources).expect("owning original container permission");
        }
    }
}
#[test]
fn opaque_vec_clone_and_root_downgrade_keep_exact_original_m7002_keywords() {
    for (case, keyword) in
        [("opaque-vec-clone-unused", "clone"), ("opaque-downgrade-unused", "downgrade")]
    {
        for provider in ["native", "typescript"] {
            let (sources, syntax) = project(case, provider);
            let Failure::Diagnostics(errors) = check_original_bodies(&syntax, &sources)
                .expect_err("opaque original denial precedes all closed replay")
            else {
                panic!("owning original diagnostic")
            };
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M7002");
            let source =
                sources.source(sources.verify_file_id(0).expect("module")).expect("source");
            let start = source
                .text()
                .find(&format!("{keyword}(value)"))
                .expect("independent original keyword");
            let span = sources
                .verify_span(UntrustedSpan {
                    file: 0,
                    start: u32::try_from(start).expect("offset"),
                    end: u32::try_from(start + keyword.len()).expect("offset"),
                })
                .expect("keyword span");
            assert_eq!(errors[0].primary_span(), Some(span));
        }
    }
}
#[test]
fn settled_source_replay_and_independent_source_proof_are_deterministic() {
    for case in SETTLED {
        for provider in ["native", "typescript"] {
            let (sources, syntax, linear, linux, claim) = claims(case, provider);
            let entry = sources.verify_file_id(0).expect("entry");
            let key = keys(case);
            let borrowed = key.iter().map(Vec::as_slice).collect::<Vec<_>>();
            assert_eq!(
                claim,
                produce_claim(&syntax, &sources, entry, &linear, &linux, &borrowed)
                    .expect("pristine deterministic replay")
            );
            let proof = verify_source(&claim, &syntax, &sources, entry, &linear, &linux, &mut 0)
                .expect("real private source verifier issued proof");
            assert_eq!(proof.world().len(), claim.type_keys.len());
            for (fi, function) in claim.functions.iter().enumerate() {
                for block in &function.blocks {
                    for (ii, instruction) in block.instructions.iter().enumerate() {
                        if let raw::Operation::Clone { source, .. } = instruction.operation {
                            let raw::PlaceOrLoan::Place(p) = source else {
                                panic!("these cases have static sources")
                            };
                            let ty = function.places[p as usize].ty;
                            assert!(
                                proof
                                    .clone_allowed(
                                        fi,
                                        block.id,
                                        u32::try_from(ii).expect("small index"),
                                        ty,
                                        &mut 0
                                    )
                                    .expect("operation-bound original permission")
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn vec_and_shared_allocation_failure_own_prepared_payloads_before_commit() {
    for provider in ["native", "typescript"] {
        for case in ["vec-owned-prepare", "shared-weak-count"] {
            let (_, _, _, _, claim) = claims(case, provider);
            let function = claim
                .functions
                .iter()
                .find(|f| f.executable_export.as_deref() == Some("entry"))
                .expect("source export");
            for block in &function.blocks {
                for instruction in &block.instructions {
                    let (values, failure) = match &instruction.operation {
                        raw::Operation::VecConstruct { elements, failure, .. } => {
                            (elements.clone(), *failure)
                        }
                        raw::Operation::SharedConstruct { value, failure, .. } => {
                            (vec![*value], *failure)
                        }
                        _ => continue,
                    };
                    let expected = values
                        .iter()
                        .rev()
                        .map(|id| {
                            let place = function
                                .places
                                .iter()
                                .find(|p| p.kind == raw::PlaceKind::Temporary(*id))
                                .expect("exact prepared affine SSA owner");
                            raw::CleanupAction::DropPlace(place.id)
                        })
                        .collect::<Vec<_>>();
                    let plan = &function.cleanup_plans[failure as usize];
                    assert_eq!(plan.span, instruction.site_span);
                    assert_eq!(
                        plan.actions, expected,
                        "allocation failure owns pending operands, no unpublished destination"
                    );
                    let result = instruction.result.as_ref().expect("container result");
                    let dest = function
                        .places
                        .iter()
                        .find(|p| p.kind == raw::PlaceKind::Temporary(result.id))
                        .expect("pending destination");
                    assert!(!plan.actions.contains(&raw::CleanupAction::DropPlace(dest.id)));
                }
            }
        }
    }
}
#[test]
fn source_phase_rejects_changed_vec_identity_cleanup_and_same_bytes_foreign_source() {
    let (sources, syntax, linear, linux, claim) = claims("vec-owned-prepare", "native");
    let entry = sources.verify_file_id(0).expect("entry");
    let mut changed = claim.clone();
    let instruction = changed
        .functions
        .iter_mut()
        .flat_map(|f| &mut f.blocks)
        .flat_map(|b| &mut b.instructions)
        .find(|i| matches!(i.operation, raw::Operation::VecConstruct { .. }))
        .expect("Vec operation");
    if let raw::Operation::VecConstruct { ty, .. } = &mut instruction.operation {
        *ty = 2;
    }
    verify_source(&changed, &syntax, &sources, entry, &linear, &linux, &mut 0)
        .expect_err("source replay rejects changed nominal/container identity before later owners");
    let mut changed = claim.clone();
    let function =
        changed.functions.iter_mut().find(|f| f.executable_export.is_some()).expect("entry");
    let failure = function
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find_map(|i| {
            if let raw::Operation::VecConstruct { failure, .. } = i.operation {
                Some(failure)
            } else {
                None
            }
        })
        .expect("Vec failure");
    function.cleanup_plans[failure as usize].actions.pop().expect("prepared owner");
    verify_source(&changed, &syntax, &sources, entry, &linear, &linux, &mut 0)
        .expect_err("omitted pending cleanup differs from exact source replay");
    let (foreign, _) = project("vec-owned-prepare", "native");
    verify_source(
        &claim,
        &syntax,
        &foreign,
        foreign.verify_file_id(0).expect("foreign entry"),
        &linear,
        &linux,
        &mut 0,
    )
    .expect_err("same bytes never launder source identity");
}
#[test]
fn container_source_exact_budget_first_extra_and_pristine_retry() {
    let (sources, syntax, linear, linux, claim) = claims("shared-weak-count", "native");
    let entry = sources.verify_file_id(0).expect("entry");
    let mut work = 0;
    verify_source(&claim, &syntax, &sources, entry, &linear, &linux, &mut work)
        .expect("owning source baseline");
    let max = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = max.checked_sub(work).expect("within inherited owner limit");
    verify_source(&claim, &syntax, &sources, entry, &linear, &linux, &mut exact)
        .expect("exact actual work credit");
    assert_eq!(exact, max);
    let mut short = max - work + 1;
    verify_source(&claim, &syntax, &sources, entry, &linear, &linux, &mut short)
        .expect_err("first extra work rejects before proof publication");
    verify_source(&claim, &syntax, &sources, entry, &linear, &linux, &mut 0)
        .expect("pristine real proof after failed attempt");
}
