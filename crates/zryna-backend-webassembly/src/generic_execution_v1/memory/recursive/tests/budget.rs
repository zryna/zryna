//! Caller-atomic real source traversal, raw interval rejection and pre-reserve accounting.
use super::*;
#[test]
fn owning_cycle_census_exact_credit_extra_overflow_and_retry_are_caller_atomic() {
    source::with_program(true, |program| {
        let mut cost = 0;
        assert!(required(program, &mut cost).expect("actual owned graph"));
        assert!(cost > program.layouts(StorageTarget::Linear32V1).types().len());
        let limit = crate::generic_execution_v1::MAX_BYTES;
        let mut exact = limit - cost;
        assert!(required(program, &mut exact).expect("exact graph credit"));
        assert_eq!(exact, limit);
        for start in [limit - cost + 1, limit, usize::MAX] {
            let mut work = start;
            work::reset_seen();
            assert!(required(program, &mut work).is_err());
            assert_eq!(work, start, "no partial caller debit or cycle answer");
            if start >= limit {
                assert_eq!(work::seen(), 0, "first debit precedes reserve");
            }
        }
        let mut retry = 0;
        assert!(required(program, &mut retry).expect("pristine source graph retry"));
        assert_eq!(retry, cost);
    });
}
#[test]
fn metadata_partition_prepays_its_header_and_all_rows_before_returning_storage() {
    source::with_program(true, |program| {
        let mut cost = 0;
        let baseline =
            partition(program, 256, true, &mut cost).expect("actual metadata and partition");
        let limit = crate::generic_execution_v1::MAX_BYTES;
        let mut exact = limit - cost;
        assert_eq!(
            partition(program, 256, true, &mut exact).expect("exact storage metadata credit"),
            baseline
        );
        assert_eq!(exact, limit);
        for start in [limit - cost + 1, usize::MAX] {
            let mut work = start;
            assert!(partition(program, 256, true, &mut work).is_err());
            assert_eq!(work, start);
        }
        for start in [MEMORY_BYTES + 1, u32::MAX, 1, 7] {
            let mut work = 0;
            assert!(partition(program, start, true, &mut work).is_err());
            assert_eq!(work, 0);
        }
        let mut retry = 0;
        assert_eq!(
            partition(program, 256, true, &mut retry).expect("pristine partition"),
            baseline
        );
        assert_eq!(retry, cost);
    });
}
#[test]
fn complete_memory_derive_and_verify_exact_budgets_publish_only_on_success() {
    source::with_program(true, |program| {
        let limit = crate::generic_execution_v1::MAX_BYTES;
        let mut derive_cost = 0;
        let baseline =
            derive_with_work(program, false, &mut derive_cost).expect("actual complete derive");
        let mut exact = limit - derive_cost;
        let derived = derive_with_work(program, false, &mut exact).expect("exact derive credit");
        assert_eq!(exact, limit);
        assert_eq!(derived.recursive, baseline.recursive);
        for start in [limit - derive_cost + 1, usize::MAX] {
            let mut work = start;
            assert!(derive_with_work(program, false, &mut work).is_err());
            assert_eq!(work, start);
        }
        let mut verify_cost = 0;
        verify_with_work(baseline.clone(), program, &mut verify_cost)
            .expect("actual independent verification");
        let mut exact = limit - verify_cost;
        verify_with_work(baseline.clone(), program, &mut exact).expect("exact verify credit");
        assert_eq!(exact, limit);
        for start in [limit - verify_cost + 1, usize::MAX] {
            let mut work = start;
            assert!(verify_with_work(baseline.clone(), program, &mut work).is_err());
            assert_eq!(work, start);
        }
        let mut retry = 0;
        verify_with_work(baseline, program, &mut retry).expect("pristine whole owner retry");
        assert_eq!(retry, verify_cost);
    });
}
#[test]
fn bad_raw_frame_products_and_aliases_reject_before_source_inventory_reserve() {
    source::with_program(true, |program| {
        let baseline = derive(program, false).expect("genuine raw claim");
        for attack in 0..4 {
            let mut claim = baseline.clone();
            let arena = claim.recursive.as_mut().expect("required arena");
            match attack {
                0 => arena.capacity = u32::MAX,
                1 => arena.metadata_start = 8,
                2 => arena.start = arena.metadata_start,
                _ => arena.metadata_bytes = u32::MAX,
            }
            work::reset_seen();
            let mut work = 0;
            assert!(verify_with_work(claim, program, &mut work).is_err());
            assert_eq!(work, 0);
            assert_eq!(work::seen(), 0);
        }
        verify(baseline, program).expect("pristine after hostile header claims");
    });
}
