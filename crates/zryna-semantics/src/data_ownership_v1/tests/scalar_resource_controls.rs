use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, with_snapshot};
use super::*;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_copy_operators::operator_fixture;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;

fn limit(values: bool) -> usize {
    if values { ir::MAX_VALUES_PER_FUNCTION } else { ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION }
}

#[test]
fn mixed_scalar_exact_and_first_extra_held_credits_charge_values_and_transitions_only() {
    // Real source, synthetic external credits: not a source program with thousands of children.
    for values in [true, false] {
        for extra in [false, true] {
            let (source, snapshot) = operator_fixture(false);
            let equality_span = snapshot.files[0].functions[0].body.expressions[7].span;
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                assert_eq!(lowerer.next_value, 0);
                assert!(lowerer.instructions.is_empty());
                let held = limit(values) - 9 + usize::from(extra);
                let mut tickets = Vec::new();
                if values {
                    for _ in 0..held {
                        tickets.push(
                            lowerer
                                .credit_ledger()
                                .acquire_constructor(0, 0)
                                .expect("checked coupled credit"),
                        );
                    }
                    assert!(held + 9 < ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION);
                } else {
                    for _ in 0..held {
                        lowerer.credit_ledger().acquire_assignment();
                    }
                }
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let storage = lowerer.preparation_storage();
                let transitions = lowerer.reserved_transitions;
                let root = root_value(lowerer, 0);
                if extra {
                    expected = Some(Diagnostic::error_at(
                        "ZRYNA-M3201",
                        span(lowerer.input.sources(), equality_span),
                        format!(
                            "derived {} exceed the per-function M3 limit of {}",
                            if values { "values" } else { "ownership transitions" },
                            limit(values)
                        ),
                        if values {
                            "reduce private aggregate expressions"
                        } else {
                            "reduce private aggregate expressions and assignments"
                        },
                    ));
                    assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_facts, facts);
                } else {
                    let prepared =
                        PreparedValue::prepare(lowerer, root, ty).expect("exact scalar frontier");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_facts, facts);
                    assert_eq!(prepared.consume(), raw::ValueId(8));
                    assert_eq!(lowerer.next_value, 9);
                    assert_eq!(lowerer.instructions.len(), 9);
                    assert_eq!(lowerer.places.len(), 3, "only String, Vec and Struct own places");
                    assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(2)]);
                    assert!(lowerer.preparation_facts.string_bytes.is_empty());
                    assert_eq!(lowerer.preparation_storage(), storage);
                    assert_eq!(lowerer.reserved_transitions, transitions);
                    assert_eq!(9 + held, limit(values));
                }
                for ticket in tickets.into_iter().rev() {
                    ticket.release(lowerer);
                }
                if !values {
                    for _ in 0..held {
                        lowerer.credit_ledger().release_assignment();
                    }
                }
                assert!(lowerer.constructor_storage_is_clear());
                assert_eq!(lowerer.reserved_transitions, 0);
            });
            assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
        }
    }
}

#[test]
fn mixed_scalar_later_semantic_error_preserves_full_state_under_exhausted_real_credits() {
    let (source, snapshot) = operator_fixture(true);
    let missing = snapshot.files[0].functions[0].body.expressions[3].span;
    for exhausted in [false, true] {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            let held = if exhausted { ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION } else { 0 };
            for _ in 0..held {
                lowerer.credit_ledger().acquire_assignment();
            }
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3002",
                span(lowerer.input.sources(), missing),
                "name 'lost' is not declared",
                "reference one exact parameter, local, or match payload binding",
            ));
            let root = root_value(lowerer, 0);
            assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
            for _ in 0..held {
                lowerer.credit_ledger().release_assignment();
            }
            assert_eq!(lowerer.reserved_transitions, 0);
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn mixed_scalar_impossible_reservation_overflow_rejects_without_mutating_real_state() {
    let (source, snapshot) = operator_fixture(false);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        // An impossible internal counter, not admitted source or a live acquired reservation.
        // The existing checked constructor ticket fails before semantic/resource replay.
        lowerer.reserved_transitions = usize::MAX;
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        let root = root_value(lowerer, 0);
        assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
        assert_eq!(state(lowerer), before);
        assert_eq!(lowerer.preparation_facts, facts);
        lowerer.reserved_transitions = 0;
    });
    assert!(errors.is_empty(), "checked internal ticket overflow is not a source diagnostic");
}
