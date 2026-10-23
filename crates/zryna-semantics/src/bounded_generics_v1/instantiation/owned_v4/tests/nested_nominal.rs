//! Frozen nested originals cross the semantic and independent v4 ownership authorities.
use super::support::{seal, with_project};
use zryna_ir::generic_v1::owned_v2::raw::Operation;
mod source;

const POSITIVE: [&str; 8] = [
    "nested-copy",
    "nested-owned-call",
    "option-box-root",
    "box-option-root",
    "plain-and-reversed-arguments",
    "phantom-remains-copy",
    "fixed-member-remains-affine",
    "nested-prefix-faults",
];
const NEGATIVE: [&str; 5] = [
    "moved-whole-twice",
    "loaned-whole-consumed",
    "affine-field-projection",
    "opaque-copy-assumption",
    "exclusive-copy-observation",
];

#[test]
fn complete_real_provider_packets_authenticate_the_same_frozen_nested_originals() {
    for case in POSITIVE.into_iter().chain(NEGATIVE) {
        let (native_sources, native) = source::project(case, "native");
        let (ts_sources, ts) = source::project(case, "typescript");
        assert!(native.is_bound_to(&native_sources));
        assert!(ts.is_bound_to(&ts_sources));
        assert_eq!(native.files(), ts.files(), "complete authenticated DTO: {case}");
    }
}

#[test]
fn contextual_nested_facts_seal_all_original_bodies_and_both_target_layouts() {
    for case in POSITIVE {
        for provider in ["native", "typescript"] {
            let (sources, syntax) = source::project(case, provider);
            with_project(&sources, &syntax, |claim, syntax, sources, linear, linux| {
                source::check_keys(case, linear, linux);
                seal(&claim, syntax, sources, linear, linux).expect("same pristine authority");
            });
        }
    }
}

#[test]
fn nested_copy_has_two_whole_constructions_two_copy_observations_and_no_cleanup() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = source::project("nested-copy", provider);
        with_project(&sources, &syntax, |claim, _, _, _, _| {
            let operations = &claim.extensions[0];
            assert_eq!(
                operations
                    .iter()
                    .filter(|e| matches!(e.operation, Operation::StructConstruct { .. }))
                    .count(),
                2
            );
            assert_eq!(
                operations
                    .iter()
                    .filter(|e| matches!(e.operation, Operation::CopyStructField { .. }))
                    .count(),
                2
            );
            assert!(
                claim.plans[0].steps.iter().all(|step| !step.failure
                    && step.cleanup.is_empty()
                    && step.end_loans.is_empty())
            );
            assert!(operations.iter().all(|e| !matches!(
                e.operation,
                Operation::StringLiteral(_) | Operation::CloneString(_)
            )));
        });
    }
}

#[test]
fn nested_construction_keeps_independent_declaration_order_and_prefix_cleanup_oracle() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = source::project("nested-prefix-faults", provider);
        with_project(&sources, &syntax, |claim, _, _, _, _| {
            source::check_prefix_claim(&claim);
        });
    }
}

#[test]
fn nested_copy_proof_does_not_launder_opaque_ownership_moves_or_exclusive_loans() {
    for case in NEGATIVE {
        for provider in ["native", "typescript"] {
            let errors = source::reject(case, provider);
            assert_eq!(errors.len(), 1, "{case}/{provider}: {errors:?}");
            assert_eq!(errors[0].code(), "ZRYNA-M7007", "{case}/{provider}");
            assert!(errors[0].primary_span().is_some());
        }
    }
}
