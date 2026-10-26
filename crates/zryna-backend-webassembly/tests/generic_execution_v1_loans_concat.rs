//! Genuine authenticated original loans and concat through the complete new execution issuer.
#[path = "generic_execution_v1_loans_concat/concat_negatives.rs"]
mod concat_negatives;
#[path = "generic_execution_v1_loans_concat/concat_source.rs"]
mod concat_source;
#[path = "generic_execution_v1_loans_concat/loan_negatives.rs"]
mod loan_negatives;
#[path = "generic_execution_v1_loans_concat/loan_source.rs"]
mod loan_source;
#[path = "generic_execution_v1_loans_concat/runtime.rs"]
mod runtime;
use zryna_backend_webassembly::generic_execution_v1::{emit, emit_observed, memory};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
const LOANS: [&str; 9] = [
    "formal-copy",
    "opaque-forward",
    "shared-reborrow-clone",
    "exclusive-reborrow-push",
    "generic-replace",
    "static-disjoint",
    "branch-cleanup",
    "fallible-cleanup",
    "referent-catalogue",
];
const CONCAT: [&str; 4] =
    ["local-observation", "same-source", "nested-temporaries", "formal-observation"];
fn program(case: &str, provider: &str, check: impl FnOnce(&VerifiedExecution<'_>, &[u8])) {
    if LOANS.contains(&case) {
        loan_source::with_program(case, provider, check);
    } else {
        assert!(CONCAT.contains(&case));
        concat_source::with_program(case, provider, check);
    }
}
#[test]
fn all_thirteen_cases_authenticate_both_complete_packets_and_issue_only_new_whole_seals() {
    for case in LOANS.into_iter().chain(CONCAT) {
        let (_, native) = if LOANS.contains(&case) {
            loan_source::project(case, "native")
        } else {
            concat_source::project(case, "native")
        };
        let (_, typescript) = if LOANS.contains(&case) {
            loan_source::project(case, "typescript")
        } else {
            concat_source::project(case, "typescript")
        };
        assert_eq!(native.files(), typescript.files(), "complete provider parity {case}");
        for provider in ["native", "typescript"] {
            program(case, provider, |program, wire| {
                assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
                assert!(!program.export_functions().is_empty());
            });
        }
    }
}
fn ordinary(case: &str) {
    for provider in ["native", "typescript"] {
        program(case, provider, |program, _| {
            let plan = memory::verify(
                memory::derive(program, false).expect("actual memory claim"),
                program,
            )
            .expect("same-program independently verified frame/loan pointer inventory");
            assert_eq!(plan.claims().memory_pages, 256);
            assert_eq!(plan.claims().scratch_bytes, 32);
            assert!(
                plan.claims().heap_start >= plan.claims().frames_start + plan.claims().frames_bytes
            );
            let artifact = emit(program).expect("genuine loan/concat full final artifact audit");
            assert_eq!(
                artifact.bytes(),
                emit(program).expect("deterministic same-seal emit").bytes()
            );
            runtime::ordinary(case, artifact.bytes());
        });
    }
}
fn observed(case: &str) {
    program(case, "native", |program, _| {
        let artifact =
            emit_observed(program).expect("distinct fully audited current observer artifact");
        runtime::observed(case, artifact.as_bytes());
    });
}
macro_rules! cases {
    ($($normal:ident, $observe:ident, $case:literal;)*) => {$(
        #[test] fn $normal() { ordinary($case); }
        #[test] fn $observe() { observed($case); }
    )*};
}
cases! {
    ordinary_formal_copy, observed_formal_copy, "formal-copy";
    ordinary_opaque_forward, observed_opaque_forward, "opaque-forward";
    ordinary_shared_reborrow_clone, observed_shared_reborrow_clone, "shared-reborrow-clone";
    ordinary_exclusive_reborrow_push, observed_exclusive_reborrow_push, "exclusive-reborrow-push";
    ordinary_generic_replace, observed_generic_replace, "generic-replace";
    ordinary_static_disjoint, observed_static_disjoint, "static-disjoint";
    ordinary_branch_cleanup, observed_branch_cleanup, "branch-cleanup";
    ordinary_fallible_cleanup, observed_fallible_cleanup, "fallible-cleanup";
    ordinary_referent_catalogue, observed_referent_catalogue, "referent-catalogue";
    ordinary_local_observation, observed_local_observation, "local-observation";
    ordinary_same_source, observed_same_source, "same-source";
    ordinary_nested_temporaries, observed_nested_temporaries, "nested-temporaries";
    ordinary_formal_observation, observed_formal_observation, "formal-observation";
}
