use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::constructor_resources::CreditLedgerMut;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn panic_text(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("string panic")
}

fn copy_type(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> Ty {
    lowerer
        .node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == TypeCategory::Bool)
        .copied()
        .expect("fixture bool")
}

#[test]
fn aggregate_credit_bridges_ordered_chains_preserve_diagnostics_and_copy_distinction() {
    for constructor in [false, true] {
        for copy in [false, true] {
            for dimension in usize::from(!constructor)..4 {
                let mut diagnostic = None;
                let accepted = !constructor && copy && dimension == 3;
                let errors = with_fixture(Fixture::Pair, |lowerer, result| {
                    let result = if copy { copy_type(lowerer) } else { result };
                    let at = span(lowerer.input.sources(), lowerer.function.span);
                    diagnostic = Some(expected(dimension, at));
                    let mut held = LIMITS.map(|limit| limit + 1);
                    held[..dimension].fill(0);
                    set_credits(lowerer, held); // Deliberate competing counter failures.
                    let view = lowerer.resource_usage();
                    let before = counters(&view);
                    let live = state(lowerer);
                    let selected = if constructor {
                        view.constructor(result, 2, at, lowerer.errors)
                    } else {
                        view.emit(result, at, lowerer.errors)
                    };
                    assert_eq!(selected, accepted);
                    assert_eq!(counters(&view), before);
                    assert_eq!(state(lowerer), live);
                });
                if accepted {
                    assert!(errors.is_empty());
                } else {
                    assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
                }
            }
        }
    }
}

#[test]
fn aggregate_credit_bridges_constructor_checked_add_failure_is_atomic() {
    // Direct ledger overflow is a helper invariant probe, not valid full IR.
    for dimension in 0..4 {
        let errors = with_fixture(Fixture::Pair, |lowerer, _| {
            let mut held = [7, 11, 13, 17];
            held[dimension] = usize::MAX;
            set_credits(lowerer, held);
            let before = state(lowerer);
            assert!(lowerer.credit_ledger().acquire_constructor(2, 1).is_none());
            assert_eq!(state(lowerer), before);
        });
        assert!(errors.is_empty(), "checked ledger arithmetic emits no diagnostics");
    }
}

#[test]
fn aggregate_credit_bridges_constructor_release_underflow_order_is_atomic() {
    let messages = [
        "held constructor place credit",
        "held constructor value credit",
        "held constructor transition credit",
        "held constructor operand credit",
    ];
    for (failure, message) in messages.into_iter().enumerate() {
        let errors = with_fixture(Fixture::Pair, |lowerer, _| {
            let ticket = lowerer.credit_ledger().acquire_constructor(2, 1).expect("ticket");
            let mut held = [2, 1, 1, 1];
            // Earlier checks succeed; this and later checks deliberately underflow.
            for index in [3, 2, 1, 0].into_iter().skip(failure) {
                held[index] = 0;
            }
            set_credits(lowerer, held);
            let before = state(lowerer);
            let panic = catch_unwind(AssertUnwindSafe(|| {
                lowerer.credit_ledger().release_constructor(ticket);
            }))
            .expect_err("corrupt ledger must retain invariant panic");
            assert_eq!(panic_text(panic.as_ref()), message);
            assert_eq!(state(lowerer), before, "no partial release before later underflow");
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn aggregate_credit_bridges_assignment_invariant_panics_preserve_counts() {
    for acquire in [false, true] {
        let errors = with_fixture(Fixture::Pair, |lowerer, _| {
            set_credits(lowerer, [7, if acquire { usize::MAX } else { 0 }, 13, 17]);
            let before = state(lowerer);
            let panic = catch_unwind(AssertUnwindSafe(|| {
                if acquire {
                    lowerer.credit_ledger().acquire_assignment();
                } else {
                    lowerer.credit_ledger().release_assignment();
                }
            }))
            .expect_err("direct ledger misuse must retain invariant panic");
            assert_eq!(
                panic_text(panic.as_ref()),
                if acquire {
                    "assignment transition capacity preflighted"
                } else {
                    "reserved aggregate assignment transition"
                }
            );
            assert_eq!(state(lowerer), before);
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn aggregate_credit_bridges_nested_tickets_share_assignment_total_and_release_owned_credits() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        let at = span(lowerer.input.sources(), lowerer.function.span);
        assert!(lowerer.reserve_transition(at));
        assert!(lowerer.constructor_storage_is_clear(), "assignment is not constructor storage");
        assert_eq!(credits(lowerer), [0, 1, 0, 0]);
        let outer = lowerer.reserve_constructor_commit(result, 2, at).expect("outer");
        let inner = lowerer.reserve_constructor_commit(result, 3, at).expect("inner");
        assert_eq!(credits(lowerer), [5, 3, 2, 2]);
        inner.release(lowerer);
        assert_eq!(credits(lowerer), [2, 2, 1, 1]);
        outer.release(lowerer);
        assert_eq!(credits(lowerer), [0, 1, 0, 0]);
        assert!(lowerer.constructor_storage_is_clear());
        lowerer.release_transition();
        assert_eq!(credits(lowerer), [0; 4]);
        assert!(lowerer.instructions.is_empty());
        assert!(lowerer.places.is_empty());
        assert!(lowerer.cleanup_plans.is_empty());
        assert!(lowerer.owners.pending().is_empty());
    });
    assert!(errors.is_empty());
}

#[test]
fn aggregate_credit_bridges_scratch_ledger_uses_same_affine_operations_without_live_effects() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let before = state(lowerer);
        let mut storage = ConstructorStorage::default();
        let mut transitions = 1;
        let mut ledger = CreditLedgerMut { storage: &mut storage, transitions: &mut transitions };
        let ticket = ledger.acquire_constructor(2, 1).expect("scratch ticket");
        ledger.acquire_assignment();
        assert_eq!(*ledger.transitions, 3);
        ledger.release_constructor(ticket);
        ledger.release_assignment();
        assert_eq!(*ledger.transitions, 1);
        assert_eq!(*ledger.storage, ConstructorStorage::default());
        assert_eq!(state(lowerer), before);
    });
    assert!(errors.is_empty());
}

#[test]
fn aggregate_credit_bridges_live_assignment_frontier_rejects_without_mutation() {
    for excess in [0, 1] {
        let mut diagnostic = None;
        let errors = with_fixture(Fixture::Pair, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let transitions = LIMITS[1] - lowerer.instructions.len() - 1 + excess;
            set_credits(lowerer, [0, transitions, 0, 0]);
            let at = span(lowerer.input.sources(), lowerer.function.span);
            diagnostic = Some(expected(1, at));
            let before = state(lowerer);
            assert_eq!(lowerer.reserve_transition(at), excess == 0);
            if excess == 0 {
                lowerer.release_transition();
            }
            assert_eq!(state(lowerer), before);
        });
        if excess == 0 {
            assert!(errors.is_empty());
        } else {
            assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
        }
    }
}
