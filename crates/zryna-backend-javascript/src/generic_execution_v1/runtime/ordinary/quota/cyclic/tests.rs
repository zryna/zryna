//! Genuine same-seal cyclic proof ownership and its own prepayment boundary.
use super::*;
use crate::generic_execution_v1::while_tests as source;
#[test]
fn genuine_cycles_use_canonical_caps_and_acyclic_entries_keep_the_existing_proof() {
    for (case, cyclic) in [
        ("copy-assignment", false),
        ("zero-multiple", true),
        ("nested-lexical", true),
        ("early-return", true),
        ("body-loan-cleanup", true),
        ("opaque-retained", true),
        ("owned-root-replacement", false),
        ("branch-copy-update", false),
        ("match-condition", true),
    ] {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, _| {
                let context = Context::new(program).expect("genuine new execution context");
                let entry = program.export_functions()[0];
                let result = prove(&context, entry).expect("closed effect/call proof");
                assert_eq!(result.is_some(), cyclic);
                if let Some(quota) = result {
                    assert_eq!(
                        quota.allocations,
                        limit(zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS)
                            .expect("canonical width")
                    );
                    assert_eq!(
                        quota.transitions,
                        limit(zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS)
                            .expect("canonical width")
                    );
                    assert!(
                        quota.allocations * 2 + 1 < quota.transitions,
                        "nonempty String cleanup cannot exhaust status credit"
                    );
                }
            });
        }
    }
}
#[test]
fn cyclic_certificate_prepays_its_indices_and_exact_work_then_retries_pristine() {
    source::with_program("nested-lexical", "native", |program, _| {
        let context = Context::new(program).expect("genuine checked types/operations");
        let entry = program.export_functions()[0];
        let limit = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES;
        context.work.set(0);
        let baseline = prove(&context, entry).expect("baseline").expect("real backedge");
        let cost = context.work.get();
        context.work.set(limit - cost);
        let exact = prove(&context, entry).expect("exact proof credit").expect("same backedge");
        assert_eq!(context.work.get(), limit);
        assert_eq!(
            (exact.allocations, exact.transitions),
            (baseline.allocations, baseline.transitions)
        );
        context.work.set(limit - cost + 1);
        assert_eq!(
            prove(&context, entry).err().expect("first missing credit").code(),
            "ZRYNA-J2003"
        );
        // The first graph-index debit must reject before any reservation, without retrofunding.
        for start in [limit - context.functions.len() + 1, usize::MAX] {
            context.work.set(start);
            assert_eq!(
                prove(&context, entry).err().expect("first debit rejects").code(),
                "ZRYNA-J2003"
            );
            assert_eq!(context.work.get(), start);
        }
        context.work.set(0);
        prove(&context, entry).expect("pristine proof after rejection").expect("same cycle");
        assert_eq!(context.work.get(), cost);
    });
}
