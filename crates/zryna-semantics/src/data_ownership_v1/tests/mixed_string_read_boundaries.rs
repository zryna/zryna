use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::cleanup_frontiers::seed_external;
use super::*;
use crate::data_ownership_v1::owned_lowering_resources::CleanupUsage;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_string_read_scopes::{ReadCase, read_fixture};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::{RawExpressionKind, RawIdentifierSyntax};

#[derive(Clone, Copy)]
enum Pressure {
    None,
    Values,
    Places,
    Transitions,
    Cleanup,
}

#[test]
fn mixed_string_invalid_right_precedes_exhausted_resources_and_keeps_ancestor_state() {
    let (mut source, mut snapshot) = read_fixture(ReadCase::NestedConcat);
    let expression = snapshot.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find(|expression| {
            matches!(&expression.kind,
            RawExpressionKind::StringLiteral { spelling } if spelling == "\"b\"")
        })
        .expect("right operand literal");
    let bad = expression.span;
    let range = usize::try_from(bad.start).expect("source offset")
        ..usize::try_from(bad.end).expect("source offset");
    assert_eq!(&source[range.clone()], "\"b\"");
    source.replace_range(range, "bad");
    expression.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "bad".into(), span: bad },
    };
    for pressure in [
        Pressure::None,
        Pressure::Values,
        Pressure::Places,
        Pressure::Transitions,
        Pressure::Cleanup,
    ] {
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
                let (plans, actions) = if matches!(pressure, Pressure::Cleanup) {
                    (ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 1, ir::MAX_DROP_ACTIONS_PER_FUNCTION - 3)
                } else {
                    (lowerer.cleanup_plans.len(), lowerer.cleanup_actions)
                };
                if !matches!(pressure, Pressure::None) {
                    seed_external(lowerer, plans, actions);
                }
                // Existing constructor credits acquire one value and one transition,
                // with independently chosen zero places/operands. At the value limit,
                // transitions remain strictly below their much larger limit.
                let value_tickets = if matches!(pressure, Pressure::Values) {
                    let tickets = (0..ir::MAX_VALUES_PER_FUNCTION)
                        .map(|_| {
                            lowerer
                                .credit_ledger()
                                .acquire_constructor(0, 0)
                                .expect("checked synthetic zero-place value credit")
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        lowerer.preparation_storage().counts(),
                        [ir::MAX_VALUES_PER_FUNCTION, 0, 0]
                    );
                    assert!(
                        lowerer.reserved_transitions < ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    );
                    tickets
                } else {
                    Vec::new()
                };
                // Real initialized local plus synthetic held-resource frontiers.
                let ticket = if matches!(pressure, Pressure::Places) {
                    Some(
                        lowerer
                            .credit_ledger()
                            .acquire_constructor(0, ir::MAX_PLACES_PER_FUNCTION)
                            .expect("checked synthetic place credit"),
                    )
                } else {
                    None
                };
                if matches!(pressure, Pressure::Transitions) {
                    lowerer.reserved_transitions = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
                }
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3002",
                    span(lowerer.input.sources(), bad),
                    "String operand 'bad' is not declared",
                    "reference one exact preceding String local",
                ));
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                assert_eq!(
                    facts.held_cleanup,
                    if matches!(pressure, Pressure::None) { [0, 0] } else { [1, 3] }
                );
                let id = root_value(lowerer, 1);
                assert!(PreparedValue::prepare(lowerer, id, ty).is_none());
                assert_eq!(state(lowerer), before, "all arenas, maps, masks, cache and credits");
                assert_eq!(lowerer.preparation_facts, facts, "live bytes and ancestor credits");
                if let Some(ticket) = ticket {
                    ticket.release(lowerer);
                }
                for ticket in value_tickets {
                    ticket.release(lowerer);
                }
            });
            assert_eq!(errors, [expected.expect("source-bound exact diagnostic")]);
        }
    }
}

fn valid_frontier(plans: usize, actions: usize, failure: Option<bool>) {
    let (source, snapshot) = read_fixture(ReadCase::LocalConcat);
    for _ in 0..2 {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            seed_external(lowerer, plans, actions);
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            let id = root_value(lowerer, 1);
            if let Some(plan_failure) = failure {
                let concat = lowerer
                    .function
                    .body
                    .expressions
                    .iter()
                    .find(|expression| {
                        matches!(&expression.kind, RawExpressionKind::Call { callee, .. }
                        if callee.text == "concat")
                    })
                    .expect("source concat")
                    .span;
                let (message, guidance) = if plan_failure {
                    (
                        format!(
                            "derived cleanup sites exceed the per-function M3 limit of {}",
                            ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                        ),
                        "reduce fallible String leaves in private aggregate construction",
                    )
                } else {
                    (
                        format!(
                            "derived cleanup actions exceed the per-function M3 limit of {}",
                            ir::MAX_DROP_ACTIONS_PER_FUNCTION
                        ),
                        "reduce simultaneously live owned aggregates and String leaves",
                    )
                };
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(lowerer.input.sources(), concat),
                    message,
                    guidance,
                ));
                assert!(PreparedValue::prepare(lowerer, id, ty).is_none());
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_facts, facts);
            } else {
                let prepared =
                    PreparedValue::prepare(lowerer, id, ty).expect("exact concat frontier");
                assert_eq!(state(prepared.lowerer), before);
                assert_eq!(prepared.lowerer.preparation_facts, facts);
                let held = prepared
                    .plan
                    .steps
                    .iter()
                    .filter_map(|step| {
                        matches!(
                            step.operation,
                            Operation::StringEnter { .. }
                                | Operation::StringRead(_)
                                | Operation::StringExit
                        )
                        .then_some(step.after.held_cleanup)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(held, vec![[2, 6]; 4], "String scope preserves exact ancestor demand");
                let value = prepared.consume();
                assert_eq!(lowerer.owners.owner(value), Some(raw::PlaceId(5)));
                assert_eq!(
                    lowerer.owners.pending(),
                    &[raw::PlaceId(1), raw::PlaceId(2), raw::PlaceId(5)]
                );
                assert_eq!(lowerer.cleanup_plans.len(), plans + 3);
                assert_eq!(lowerer.cleanup_actions, actions + 6);
                assert_eq!(lowerer.preparation_facts.held_cleanup, [1, 3]);
                assert!(lowerer.constructor_storage_is_clear());
                lowerer.preparation_facts.held_cleanup = CleanupUsage::release([1, 3], 3);
                assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
            }
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn mixed_string_valid_summary_exact_extra_cleanup_keeps_phase_context_and_external_credits() {
    // This is a compiler counter boundary, not a source-sized/runtime allocation claim.
    let plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION;
    valid_frontier(plans - 4, actions - 9, None);
    valid_frontier(plans - 3, 0, Some(true));
    valid_frontier(1, actions - 8, Some(false));
    valid_frontier(plans - 3, actions - 8, Some(true));
}
