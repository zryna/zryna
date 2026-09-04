use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::nested_mixed_construction::local_commit_fixture::local_commit_fixture;
use crate::data_ownership_v1::tests::nested_mixed_construction::local_tail_supplement::tail_fixture;
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;

#[test]
fn mixed_local_tail_competing_destination_limits_choose_place_without_consuming_initializer() {
    let (source, raw) = local_commit_fixture(false);
    let statement = raw.files[0].functions[0].body.statements[1].span;
    let mut expected = None;
    let errors = with_snapshot(&source, raw, |lowerer, ty| {
        assert!(run_statement(lowerer, 0, ty));
        assert_eq!(lowerer.instructions.len(), 4);
        assert_eq!(lowerer.places.len(), 4);
        let ticket = lowerer
            .credit_ledger()
            .acquire_constructor(0, ir::MAX_PLACES_PER_FUNCTION - 5)
            .expect("place credit");
        // The constructor ticket already owns one transition credit.
        let assignments = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 6;
        for _ in 0..assignments {
            lowerer.credit_ledger().acquire_assignment();
        }
        let before = state(lowerer);
        let facts = lowerer.preparation_facts.clone();
        expected = Some(Diagnostic::error_at(
            "ZRYNA-M3201",
            span(lowerer.input.sources(), statement),
            "derived aggregate places exceed the per-function M3 limit",
            "reduce private aggregate locals",
        ));
        assert!(!run_statement(lowerer, 1, ty));
        assert_eq!(state(lowerer), before);
        assert_eq!(lowerer.preparation_facts, facts);
        for _ in 0..assignments {
            lowerer.credit_ledger().release_assignment();
        }
        ticket.release(lowerer);
        assert!(lowerer.constructor_storage_is_clear());
        assert_eq!(lowerer.reserved_transitions, 0);
    });
    assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn mixed_local_tail_copy_owns_no_temporary_but_still_charges_destination_and_string_renames_facts()
{
    // Synthetic acquired surrounding capacity, around authenticated ordinary small sources.
    for copy in [true, false] {
        for extra in [false, true] {
            let (source, raw) = tail_fixture(copy);
            let statement = raw.files[0].functions[0].body.statements[0].span;
            let mut expected = None;
            let errors = with_snapshot(&source, raw, |lowerer, ty| {
                let places = if copy { 1 } else { 2 };
                let held = ir::MAX_PLACES_PER_FUNCTION - places + usize::from(extra);
                let ticket = lowerer
                    .credit_ledger()
                    .acquire_constructor(0, held)
                    .expect("real surrounding place credit");
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let storage = lowerer.preparation_storage();
                let transitions = lowerer.reserved_transitions;
                if extra {
                    expected = Some(Diagnostic::error_at(
                        "ZRYNA-M3201",
                        span(lowerer.input.sources(), statement),
                        "derived aggregate places exceed the per-function M3 limit",
                        "reduce private aggregate locals",
                    ));
                    assert!(!run_statement(lowerer, 0, ty));
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_facts, facts);
                } else {
                    assert!(run_statement(lowerer, 0, ty));
                    assert_eq!(lowerer.next_value, 1);
                    assert_eq!(lowerer.instructions.len(), 2);
                    assert_eq!(lowerer.places.len(), places);
                    assert_eq!(lowerer.next_local, 1);
                    assert_eq!(lowerer.bindings.len(), 1);
                    let local = raw::PlaceId(u32::from(!copy));
                    assert_eq!(lowerer.bindings[if copy { "count" } else { "text" }].place, local);
                    assert_eq!(lowerer.owners.pending(), if copy { vec![] } else { vec![local] });
                    assert_eq!(
                        lowerer.preparation_facts.string_bytes,
                        if copy { BTreeMap::new() } else { BTreeMap::from([(local, 1)]) }
                    );
                    assert!(
                        !lowerer.preparation_facts.string_bytes.contains_key(&raw::PlaceId(0)),
                        "no stale String temporary fact"
                    );
                    assert_eq!(lowerer.preparation_storage(), storage);
                    assert_eq!(lowerer.reserved_transitions, transitions);
                    assert_eq!(places + held, ir::MAX_PLACES_PER_FUNCTION);
                }
                ticket.release(lowerer);
                assert!(lowerer.constructor_storage_is_clear());
                assert_eq!(lowerer.reserved_transitions, 0);
            });
            assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
        }
    }
}
