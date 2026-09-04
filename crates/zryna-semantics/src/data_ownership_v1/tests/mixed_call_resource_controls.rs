use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::cleanup_frontiers::seed_external;
use super::*;
use crate::data_ownership_v1::owned_lowering_resources::CleanupUsage;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_string_calls::mixed_string_call_fixture;
use crate::data_ownership_v1::tests::mixed_vec_calls::mixed_vec_call_fixture;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::{RawExpressionKind, RawProjectSyntaxSnapshot};

#[derive(Clone, Copy)]
enum Family {
    String,
    Vec,
}
#[derive(Clone, Copy)]
enum Dimension {
    Values,
    Places,
    Transitions,
}

fn fixture(family: Family) -> (String, RawProjectSyntaxSnapshot) {
    match family {
        Family::String => mixed_string_call_fixture(),
        Family::Vec => mixed_vec_call_fixture(),
    }
}

fn setup(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, ty: Ty, family: Family) -> u32 {
    if matches!(family, Family::String) {
        assert!(run_statement(lowerer, 0, ty));
        assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
        root_value(lowerer, 1)
    } else {
        assert!(lowerer.owners.pending().is_empty());
        root_value(lowerer, 0)
    }
}

fn maximum(dimension: Dimension) -> usize {
    match dimension {
        Dimension::Values => ir::MAX_VALUES_PER_FUNCTION,
        Dimension::Places => ir::MAX_PLACES_PER_FUNCTION,
        Dimension::Transitions => ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
    }
}

fn peak(family: Family, dimension: Dimension) -> usize {
    match (family, dimension) {
        (Family::String, Dimension::Values) => 5,
        (Family::String, _) => 6,
        (Family::Vec, _) => 3,
    }
}

fn capacity_diagnostic(dimension: Dimension) -> (String, &'static str) {
    let (label, help) = match dimension {
        Dimension::Values => (
            "owned CFG values",
            "reduce owned function parameters, block parameters, and result-producing expressions",
        ),
        Dimension::Places => {
            ("derived places", "reduce owned parameters, expressions, and local declarations")
        }
        Dimension::Transitions => {
            ("owned CFG transitions", "reduce owned operations before control-flow lowering")
        }
    };
    (format!("{label} exceed the per-function M3 limit of {}", maximum(dimension)), help)
}

fn call_span(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, family: Family) -> Span {
    let name = if matches!(family, Family::String) { "identity" } else { "producer" };
    let expression = lowerer
        .function
        .body
        .expressions
        .iter()
        .find(|e| {
            matches!(&e.kind,
        RawExpressionKind::Call { callee, .. } if callee.text == name)
        })
        .expect("source call frontier");
    span(lowerer.input.sources(), expression.span)
}

fn capacity_case(family: Family, dimension: Dimension, extra: bool) {
    let (source, snapshot) = fixture(family);
    let mut expected = None;
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        let root = setup(lowerer, ty, family);
        let baseline_cleanup = (lowerer.cleanup_plans.len(), lowerer.cleanup_actions);
        seed_external(lowerer, baseline_cleanup.0, baseline_cleanup.1);
        let held = maximum(dimension) - peak(family, dimension) + usize::from(extra);
        let mut tickets = Vec::new();
        let assignments = if matches!(dimension, Dimension::Transitions) { held } else { 0 };
        match dimension {
            Dimension::Values => {
                for _ in 0..held {
                    tickets.push(
                        lowerer
                            .credit_ledger()
                            .acquire_constructor(0, 0)
                            .expect("checked value/transition ticket"),
                    );
                }
                assert!(
                    lowerer.reserved_transitions + 6 < ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                );
            }
            Dimension::Places => tickets.push(
                lowerer.credit_ledger().acquire_constructor(0, held).expect("checked place ticket"),
            ),
            Dimension::Transitions => {
                for _ in 0..assignments {
                    lowerer.credit_ledger().acquire_assignment();
                }
            }
        }
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        let storage = lowerer.preparation_storage();
        let transitions = lowerer.reserved_transitions;
        if extra {
            let (message, diagnostic_help) = capacity_diagnostic(dimension);
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3201",
                call_span(lowerer, family),
                message,
                diagnostic_help,
            ));
            assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
        } else {
            let prepared = PreparedValue::prepare(lowerer, root, ty).expect("exact call frontier");
            assert_eq!(state(prepared.lowerer), before);
            assert_eq!(prepared.lowerer.preparation_facts, facts);
            let result = prepared.consume();
            assert_eq!(
                lowerer.owners.owner(result),
                Some(raw::PlaceId(if matches!(family, Family::String) { 5 } else { 2 }))
            );
            assert_eq!(lowerer.preparation_storage(), storage);
            assert_eq!(lowerer.reserved_transitions, transitions);
            assert_eq!(lowerer.preparation_facts.held_cleanup, [1, 3]);
            let actual = match dimension {
                Dimension::Values => lowerer.next_value as usize + storage.counts()[0],
                Dimension::Places => lowerer.places.len() + storage.counts()[1],
                Dimension::Transitions => lowerer.instructions.len() + lowerer.reserved_transitions,
            };
            assert_eq!(actual, maximum(dimension), "actual committed+held exact frontier");
        }
        for ticket in tickets.into_iter().rev() {
            ticket.release(lowerer);
        }
        for _ in 0..assignments {
            lowerer.credit_ledger().release_assignment();
        }
        lowerer.preparation_facts.held_cleanup =
            CleanupUsage::release(lowerer.preparation_facts.held_cleanup, 3);
        assert!(lowerer.constructor_storage_is_clear());
        assert_eq!(lowerer.reserved_transitions, 0);
        assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn mixed_calls_exact_and_first_extra_real_ledger_credits_preserve_external_reservations() {
    // Synthetic compiler capacity credits around real authenticated sources, not huge programs.
    for family in [Family::String, Family::Vec] {
        for dimension in [Dimension::Values, Dimension::Places, Dimension::Transitions] {
            for extra in [false, true] {
                capacity_case(family, dimension, extra);
            }
        }
    }
}

fn cleanup_case(family: Family, plans: usize, actions: usize, reject: bool) {
    let (source, snapshot) = fixture(family);
    let mut expected = None;
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        let root = setup(lowerer, ty, family);
        seed_external(lowerer, plans, actions);
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        if reject {
            // The first identity preflight owns its subtree estimate and sees ancestor credits.
            let expression = lowerer
                .function
                .body
                .expressions
                .iter()
                .find(|e| {
                    matches!(&e.kind,
                RawExpressionKind::Call { callee, .. } if callee.text == "identity")
                })
                .expect("identity");
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3201",
                span(lowerer.input.sources(), expression.span),
                "recursive owned String preparation exceeds the per-function cleanup limits",
                "reduce nested String-producing expressions or simultaneously live owners",
            ));
            assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
        } else {
            let prepared =
                PreparedValue::prepare(lowerer, root, ty).expect("exact recorded cleanup envelope");
            assert_eq!(state(prepared.lowerer), before);
            assert_eq!(prepared.lowerer.preparation_facts, facts);
            let expected_peak = if matches!(family, Family::String) { [4, 7] } else { [3, 3] };
            assert!(
                prepared.plan.steps.iter().any(|s| s.after.held_cleanup == expected_peak),
                "nested call owns distinct cleanup reservation"
            );
            prepared.consume();
            let (new_plans, new_actions) =
                if matches!(family, Family::String) { (3, 4) } else { (2, 0) };
            assert_eq!(lowerer.cleanup_plans.len(), plans + new_plans);
            assert_eq!(lowerer.cleanup_actions, actions + new_actions);
            assert_eq!(lowerer.preparation_facts.held_cleanup, [1, 3]);
            assert_eq!(lowerer.cleanup_plans.len() + 1, ir::MAX_CLEANUP_PLANS_PER_FUNCTION);
            assert_eq!(lowerer.cleanup_actions + 3, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
            assert!(lowerer.constructor_storage_is_clear());
        }
        lowerer.preparation_facts.held_cleanup =
            CleanupUsage::release(lowerer.preparation_facts.held_cleanup, 3);
        assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn mixed_calls_recorded_cleanup_exact_extra_and_zero_action_vec_keep_ancestor_credits() {
    let plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION;
    cleanup_case(Family::String, plans - 4, actions - 7, false);
    cleanup_case(Family::String, plans - 3, 0, true);
    cleanup_case(Family::String, 1, actions - 6, true);
    cleanup_case(Family::String, plans - 3, actions - 6, true);
    cleanup_case(Family::Vec, plans - 3, actions - 3, false);
    cleanup_case(Family::Vec, plans - 2, 0, true);
    // This Vec fixture has zero call cleanup actions: exact external action capacity is tested,
    // but an internal first-extra action claim would require a real surviving-owner fixture.
}
