//! Actual borrowed-match original authority, before any closed program or layout.
use super::*;
const GROUP: &str = "m7-generic-execution-borrowed-match";
const POSITIVE: [&str; 11] = [
    "nominal-shared-copy",
    "nominal-shared-clone",
    "nominal-exclusive-copy",
    "nominal-exclusive-replace",
    "option-shared-both",
    "result-shared-opaque-bound",
    "result-exclusive-both",
    "opaque-payload-forward",
    "nested-exclusive-result",
    "branch-arm-cleanup",
    "shared-reborrow-from-exclusive",
];
#[test]
fn all_frozen_borrowed_match_originals_precede_closed_layouts() {
    for case in POSITIVE {
        for provider in ["native", "typescript"] {
            check_case(GROUP, case, provider).expect("universal original arm-loan permissions");
        }
    }
}
#[test]
fn borrowed_match_original_budget_exact_extra_overflow_atomic_and_retry() {
    for case in [
        "nominal-exclusive-replace",
        "result-shared-opaque-bound",
        "nested-exclusive-result",
        "branch-arm-cleanup",
        "shared-reborrow-from-exclusive",
    ] {
        for provider in ["native", "typescript"] {
            let (sources, syntax) = project(GROUP, case, provider);
            let mut setup = 0;
            let inventory = inventory(&syntax, &sources, &mut setup).expect("actual inventory");
            let originals = authenticate(&inventory, &syntax, &mut setup).expect("actual binder");
            let mut cost = 0;
            check(&originals, &sources, &mut cost).expect("original borrowed-match baseline");
            let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
            let mut exact = limit - cost;
            check(&originals, &sources, &mut exact).expect("exact match source credit");
            assert_eq!(exact, limit);
            for mut short in [limit - cost + 1, usize::MAX] {
                let before = short;
                check(&originals, &sources, &mut short).expect_err("missing match source credit");
                assert_eq!(short, before, "no completed debit or arm state publication");
            }
            let mut retry = 0;
            check(&originals, &sources, &mut retry).expect("pristine arm-loan retry");
            assert_eq!(retry, cost, "no retained path or hidden parent survives invocation");
        }
    }
}
