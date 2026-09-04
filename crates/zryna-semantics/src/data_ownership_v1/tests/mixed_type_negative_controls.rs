use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::tests::mixed_string_calls::mixed_string_call_fixture;
use crate::data_ownership_v1::tests::nested_mixed_construction::type_negatives::{
    TypeFailure, expected_type_failure, type_fixture,
};
use crate::data_ownership_v1::tests::scalar_operator_matrix::nested_scalar_fixture;

#[test]
fn mixed_source_exact_type_failures_preserve_complete_state_facts_and_external_credit() {
    for case in
        [TypeFailure::Nominal, TypeFailure::Element, TypeFailure::Context, TypeFailure::Payload]
    {
        let (source, snapshot, at) = type_fixture(case, true);
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            let ticket =
                lowerer.credit_ledger().acquire_constructor(2, 1).expect("real surrounding ticket");
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            expected = Some(expected_type_failure(case, lowerer.input.sources(), at));
            let root = root_value(lowerer, 0);
            assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
            ticket.release(lowerer);
            assert!(lowerer.constructor_storage_is_clear());
            assert_eq!(lowerer.reserved_transitions, 0);
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn mixed_nested_scalar_and_call_plans_pin_actual_single_visit_and_result_step_counts() {
    // These fixed source rows supplement, not replace, their source/full-IR replay tests.
    for call in [false, true] {
        let (source, snapshot) =
            if call { mixed_string_call_fixture() } else { nested_scalar_fixture() };
        for _ in 0..2 {
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                if call {
                    assert!(run_statement(lowerer, 0, ty));
                }
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let root = root_value(lowerer, usize::from(call));
                let prepared =
                    PreparedValue::prepare(lowerer, root, ty).expect("authenticated nested plan");
                assert_eq!(state(prepared.lowerer), before);
                assert_eq!(prepared.lowerer.preparation_facts, facts);
                let (visits, steps, commits) = if call { (4, 16, 2) } else { (14, 25, 5) };
                assert_eq!(prepared.plan.visits, visits);
                assert_eq!(prepared.plan.steps.len(), steps);
                assert_eq!(
                    prepared.plan.steps.iter().filter(|s| s.value.is_some()).count(),
                    visits
                );
                assert_eq!(
                    prepared
                        .plan
                        .steps
                        .iter()
                        .filter(|s| if call {
                            matches!(s.operation, Operation::CallCommit { .. })
                        } else {
                            matches!(s.operation, Operation::ScalarCommit { .. })
                        })
                        .count(),
                    commits
                );
                assert_eq!(
                    prepared
                        .plan
                        .steps
                        .iter()
                        .filter(|s| matches!(s.operation, Operation::Prefix { .. }))
                        .count(),
                    0
                );
                let result = prepared.plan.result;
                assert_eq!(prepared.consume(), result);
                assert!(lowerer.constructor_storage_is_clear());
                assert_eq!(lowerer.reserved_transitions, 0);
            });
            assert!(errors.is_empty());
        }
    }
}
