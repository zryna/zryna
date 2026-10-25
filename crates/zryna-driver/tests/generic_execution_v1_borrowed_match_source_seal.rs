//! Genuine borrowed-match source permission and independent NEW whole authority.
#[path = "generic_execution_v1_borrowed_match_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_borrowed_match_source_seal/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::raw;
fn whole(case: &str) {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("universal original borrowed-match roles precede closed types");
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let mut borrowed = 0;
            for function in program.functions() {
                for block in function.blocks() {
                    if let raw::Terminator::ClosedEnumMatch { mode, scrutinee, arms, .. } =
                        block.terminator()
                        && *mode != raw::MatchMode::Value
                    {
                        borrowed += 1;
                        assert!(matches!(scrutinee, raw::PlaceOrLoan::Loan(_)));
                        for arm in arms {
                            assert!(
                                !matches!(arm.binding, Some(raw::MatchBinding::Value(_))),
                                "borrowed payload is a loan authority, never an owned SSA value"
                            );
                        }
                    }
                }
            }
            assert!(borrowed > 0, "the genuine borrowed source retains borrowed match authority");
        });
    }
}
#[test]
fn nominal_shared_copy() {
    whole("nominal-shared-copy");
}
#[test]
fn nominal_shared_clone() {
    whole("nominal-shared-clone");
}
#[test]
fn nominal_exclusive_copy() {
    whole("nominal-exclusive-copy");
}
#[test]
fn nominal_exclusive_replace() {
    whole("nominal-exclusive-replace");
}
#[test]
fn option_shared_both() {
    whole("option-shared-both");
}
#[test]
fn result_shared_opaque_bound() {
    whole("result-shared-opaque-bound");
}
#[test]
fn result_exclusive_both() {
    whole("result-exclusive-both");
}
#[test]
fn opaque_payload_forward() {
    whole("opaque-payload-forward");
}
#[test]
fn nested_exclusive_result() {
    whole("nested-exclusive-result");
}
#[test]
fn branch_arm_cleanup() {
    whole("branch-arm-cleanup");
}
#[test]
fn shared_reborrow_from_exclusive() {
    whole("shared-reborrow-from-exclusive");
}
