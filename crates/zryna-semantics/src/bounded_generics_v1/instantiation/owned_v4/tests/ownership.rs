//! Original Copy facts, complete-owner moves and live root loans remain separate proofs.
use super::support::{PROVIDERS, original_rejection, project, seal, with_claim};
use zryna_ir::generic_v1::owned_v2 as owned;

#[test]
fn affine_struct_copy_field_keeps_the_complete_owner_and_shared_loan_live() {
    for provider in PROVIDERS {
        for case in ["fixed-copy", "moved-copy", "shared-copy"] {
            with_claim(case, provider, |claim, syntax, sources, linear, linux| {
                let fields = claim
                    .extensions
                    .iter()
                    .flatten()
                    .filter_map(|extension| {
                        if let owned::raw::Operation::StructConstruct { fields, .. } =
                            &extension.operation
                        {
                            Some(fields.as_slice())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    fields,
                    vec![&[1, 0][..]],
                    "declaration order despite reordered property spelling"
                );
                let observations = claim
                    .extensions
                    .iter()
                    .flatten()
                    .filter_map(|extension| {
                        if let owned::raw::Operation::CopyStructField { value, ordinal } =
                            extension.operation
                        {
                            Some((value, ordinal))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let expected_base = if case == "moved-copy" { 3 } else { 2 };
                assert_eq!(observations, vec![(expected_base, 1)]);
                let final_step =
                    claim.plans[0].steps.last().expect("exact successful return cleanup");
                assert!(!final_step.failure);
                assert_eq!(
                    final_step.cleanup,
                    vec![expected_base],
                    "Copy observation cannot consume the String-bearing whole"
                );
                if case == "shared-copy" {
                    assert_eq!(
                        final_step.end_loans,
                        vec![3],
                        "observation cannot erase the shared loan"
                    );
                }
                seal(&claim, syntax, sources, linear, linux)
                    .expect("same pristine source authority");
            });
        }
    }
}

#[test]
fn unused_opaque_projection_and_moved_or_exclusively_loaned_bases_reject() {
    for provider in PROVIDERS {
        with_claim("opaque-identity", provider, |_, _, _, _, _| {});
        for case in ["opaque-projection", "use-after-move", "exclusive-copy"] {
            let errors = original_rejection(case, provider);
            assert_eq!(errors.len(), 1, "{case}/{provider}: {errors:?}");
            assert_eq!(errors[0].code, "ZRYNA-M7007", "{case}/{provider}: {errors:?}");
            assert!(errors[0].primary_span().is_some(), "ownership retains original use-site span");
        }
    }
}

#[test]
fn old_source_issuers_stay_closed_while_v4_seals_unused_nested_originals() {
    for provider in PROVIDERS {
        with_claim("plain-enum", provider, |_, _, _, _, _| {});
        for case in ["construction", "fixed-copy", "opaque-identity"] {
            let (sources, syntax) = project(case, provider);
            assert!(
                owned::check_original_bodies(&syntax, &sources).is_err(),
                "legacy issuer: {case}/{provider}"
            );
        }
        let (sources, syntax) = project("nested-unused", provider);
        assert!(
            owned::check_original_bodies(&syntax, &sources).is_err(),
            "the old issuer retains its original nested/Struct exclusion"
        );
        with_claim("nested-unused", provider, |claim, syntax, sources, linear, linux| {
            assert_eq!(
                claim.graph.type_keys,
                vec![
                    vec![0],
                    vec![1],
                    vec![2],
                    vec![0x10, 0, 0, 0, 0, 1, 0, 0, 0],
                    vec![0x11, 0, 0, 0, 0, 0, 0, 0, 0],
                ],
                "unused Wrapper and its Inner enum remain exact original stored roots"
            );
            seal(&claim, syntax, sources, linear, linux)
                .expect("v4 independently seals the same preserved unused originals");
        });
    }
}
