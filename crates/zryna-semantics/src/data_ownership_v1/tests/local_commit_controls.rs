use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::nested_mixed_construction::local_commit_fixture::local_commit_fixture;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;

fn exercise(places: bool, extra: bool, invalid: bool) {
    let (source, snapshot) = local_commit_fixture(invalid);
    let statement_at = snapshot.files[0].functions[0].body.statements[1].span;
    let operand_at = snapshot.files[0].functions[0].body.expressions[5].span;
    let (mut expected, mut outcome, mut comparison) = (None, None, None);
    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty), "real preceding String/Vec/Struct local");
        assert_eq!(lowerer.next_value, 3);
        assert_eq!(lowerer.instructions.len(), 4);
        assert_eq!(lowerer.places.len(), 4);
        assert_eq!(lowerer.next_local, 1);
        assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(3)]);
        let maximum = if places {
            ir::MAX_PLACES_PER_FUNCTION
        } else {
            ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
        };
        let held = maximum - 6 + usize::from(extra);
        let ticket = if places {
            Some(
                lowerer
                    .credit_ledger()
                    .acquire_constructor(0, held)
                    .expect("checked external place ticket"),
            )
        } else {
            for _ in 0..held {
                lowerer.credit_ledger().acquire_assignment();
            }
            None
        };
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        let storage = lowerer.preparation_storage();
        let transitions = lowerer.reserved_transitions;
        if invalid {
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3002",
                span(lowerer.input.sources(), operand_at),
                "aggregate value 'lost' is not declared",
                "reference one exact preceding local using its declared spelling",
            ));
        } else if extra {
            expected = Some(if places {
                Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(lowerer.input.sources(), statement_at),
                    "derived aggregate places exceed the per-function M3 limit",
                    "reduce private aggregate locals",
                )
            } else {
                Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(lowerer.input.sources(), statement_at),
                    format!(
                        "derived ownership transitions exceed the per-function M3 limit of {maximum}"
                    ),
                    "reduce private aggregate expressions and assignments",
                )
            });
        }
        outcome = Some(run_statement(lowerer, 1, ty));
        if invalid || extra {
            // Record first; compare full diagnostics before the intended red state assertion.
            comparison =
                Some((before, state(lowerer), facts.clone(), lowerer.preparation_facts.clone()));
        } else {
            assert_eq!(lowerer.next_value, 4);
            assert_eq!(lowerer.instructions.len(), 6);
            assert_eq!(lowerer.places.len(), 6);
            assert_eq!(lowerer.next_local, 2);
            assert_eq!(lowerer.bindings.len(), 2);
            assert_eq!(lowerer.bindings["copy"].place, raw::PlaceId(5));
            assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(5)]);
            assert_eq!(lowerer.preparation_storage(), storage);
            assert_eq!(lowerer.reserved_transitions, transitions);
            assert_eq!(lowerer.preparation_facts, facts);
            assert_eq!(6 + held, maximum);
        }
        if let Some(ticket) = ticket {
            ticket.release(lowerer);
        }
        if !places {
            for _ in 0..held {
                lowerer.credit_ledger().release_assignment();
            }
        }
        assert!(lowerer.constructor_storage_is_clear());
        assert_eq!(lowerer.reserved_transitions, 0);
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    assert_eq!(outcome, Some(!(invalid || extra)));
    if let Some((before, after, facts_before, facts_after)) = comparison {
        assert_eq!(
            after, before,
            "whole local statement failure preserves prior arenas, owners, bindings, cache and credits"
        );
        assert_eq!(facts_after, facts_before);
    }
}

#[test]
fn mixed_local_statement_destination_exact_place_and_transition_credits_commit_once() {
    // Acquired capacity credits around real source, not a huge source-program frontier.
    for places in [true, false] {
        exercise(places, false, false);
    }
}

#[test]
fn mixed_local_statement_destination_first_extra_place_preserves_complete_prior_state() {
    exercise(true, true, false);
}

#[test]
fn mixed_local_statement_destination_first_extra_transition_preserves_complete_prior_state() {
    exercise(false, true, false);
}

#[test]
fn mixed_local_statement_invalid_initializer_precedes_destination_capacity_without_mutation() {
    for places in [true, false] {
        for extra in [false, true] {
            exercise(places, extra, true);
        }
    }
}
