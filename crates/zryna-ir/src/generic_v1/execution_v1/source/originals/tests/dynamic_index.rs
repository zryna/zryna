//! Genuine original indexed permissions precede generated layouts and raw claims.
use super::*;
const GROUP: &str = "m7-generic-execution-dynamic-index";
const POSITIVE: [&str; 10] = [
    "copy-read-control",
    "array-shared-copy",
    "vec-shared-clone",
    "array-exclusive-copy",
    "vec-exclusive-replace",
    "vec-indexed-replace",
    "array-indexed-rhs-match",
    "index-once-before-rhs",
    "chained-shared-copy",
    "zero-length-bounds",
];
#[test]
fn dynamic_index_originals_use_source_element_and_conservative_alias_roles() {
    for case in POSITIVE {
        for provider in ["native", "typescript"] {
            check_case(GROUP, case, provider).expect("actual indexed original permissions");
        }
    }
}
#[test]
fn indexed_original_budget_exact_extra_overflow_atomic_and_retry() {
    for case in ["array-indexed-rhs-match", "index-once-before-rhs", "chained-shared-copy"] {
        for provider in ["native", "typescript"] {
            let (sources, syntax) = project(GROUP, case, provider);
            let mut setup = 0;
            let inventory = inventory(&syntax, &sources, &mut setup).expect("actual inventory");
            let originals = authenticate(&inventory, &syntax, &mut setup).expect("source binder");
            let mut cost = 0;
            check(&originals, &sources, &mut cost).expect("indexed source baseline");
            let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
            let mut exact = limit - cost;
            check(&originals, &sources, &mut exact).expect("exact indexed credit");
            assert_eq!(exact, limit);
            for mut short in [limit - cost + 1, usize::MAX] {
                let before = short;
                check(&originals, &sources, &mut short).expect_err("insufficient indexed credit");
                assert_eq!(short, before, "no debit or retained access publication");
            }
            let mut retry = 0;
            check(&originals, &sources, &mut retry).expect("pristine indexed retry");
            assert_eq!(retry, cost, "no transient target survives invocation");
        }
    }
}
