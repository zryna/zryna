use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::owned_lowering_resources::{
    CleanupUsage, OwnedCleanupReservationContext,
};
use crate::data_ownership_v1::tests::{
    constructor_envelope_fixtures as fixtures,
    mixed_phase_fixtures::{PhaseChild, phase_fixture},
};
use crate::data_ownership_v1::{self as ownership, span};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{self as ir, VerifiedInstructionKind, VerifiedModule};
use zryna_syntax::v4::{RawExpressionKind, verify_snapshot};

#[derive(Clone, Copy)]
enum Pressure {
    Places,
    Transitions,
    Cleanup,
}

fn invalid_child(mode: PhaseChild, cached: bool) {
    let (source, snapshot) = phase_fixture(mode, true);
    for pressure in [Pressure::Places, Pressure::Transitions, Pressure::Cleanup] {
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                assert_eq!(lowerer.owners.pending().len(), 1, "genuine initialized Pair root");
                let before_cache = lowerer.projections.len();
                if cached {
                    let id = lowerer
                        .function
                        .body
                        .expressions
                        .iter()
                        .position(|e| matches!(e.kind, RawExpressionKind::FieldAccess { .. }))
                        .and_then(|i| u32::try_from(i).ok())
                        .expect("actual p.first expression");
                    lowerer.owned_place(id).expect("real canonical prior projection");
                    assert_eq!(lowerer.projections.len(), before_cache + 1);
                } else {
                    assert_eq!(before_cache, 0);
                }
                // Synthetic frontier setup after real owners/cache. This is not a source
                // proof that the program can reach the configured maximum resource count.
                let ticket = if matches!(pressure, Pressure::Places) {
                    Some(
                        lowerer
                            .credit_ledger()
                            .acquire_constructor(0, ir::MAX_PLACES_PER_FUNCTION)
                            .expect("checked synthetic held-place ticket"),
                    )
                } else {
                    None
                };
                if matches!(pressure, Pressure::Transitions) {
                    lowerer.reserved_transitions = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
                }
                if matches!(pressure, Pressure::Cleanup) {
                    lowerer.preparation_facts.held_cleanup = CleanupUsage {
                        plans: lowerer.cleanup_plans.len(),
                        actions: lowerer.cleanup_actions,
                        reserved_plans: 0,
                        reserved_actions: 0,
                    }
                    .reserve(
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                        OwnedCleanupReservationContext::Vec,
                        span(lowerer.input.sources(), lowerer.function.span),
                        lowerer.errors,
                    )
                    .expect("checked external action reservation");
                }
                let bad = lowerer.function.body.expressions.iter().find(|e|
                    matches!(&e.kind, RawExpressionKind::Reference { name } if name.text == "bad"))
                    .expect("later source child").span;
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3002",
                    span(lowerer.input.sources(), bad),
                    "aggregate value 'bad' is not declared",
                    "reference one exact preceding local using its declared spelling",
                ));
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let id = root_value(lowerer, 1);
                assert!(PreparedValue::prepare(lowerer, id, ty).is_none());
                assert_eq!(
                    state(lowerer),
                    before,
                    "all arenas/cache/owners/masks/credits unchanged"
                );
                assert_eq!(
                    lowerer.preparation_facts, facts,
                    "byte facts and held cleanup unchanged"
                );
                if let Some(ticket) = ticket {
                    ticket.release(lowerer);
                }
            });
            assert_eq!(errors, [expected.expect("exact later semantic diagnostic")]);
        }
    }
}

#[test]
fn mixed_phase_uncached_and_cached_projection_later_error_precedes_resources() {
    for cached in [false, true] {
        invalid_child(PhaseChild::Projection, cached);
    }
}

#[test]
fn mixed_phase_projected_string_clone_later_error_precedes_resources() {
    for cached in [false, true] {
        invalid_child(PhaseChild::StringClone, cached);
    }
}

#[test]
fn mixed_phase_supported_aggregate_clone_later_error_precedes_resources() {
    invalid_child(PhaseChild::AggregateClone, false);
}

fn valid_source() {
    let (source, snapshot) = phase_fixture(PhaseChild::AggregateClone, false);
    let sources = fixtures::sources(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated mixed clone source");
    let mut previous = None;
    for _ in 0..2 {
        let program = ownership::lower(fixtures::input(&syntax, &sources))
            .expect("valid summary passes mandatory full independent IR verifier");
        let functions = program.modules().flat_map(VerifiedModule::functions).collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let blocks = functions[0].blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 1);
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        let kinds = instructions.iter().map(|i| i.kind()).collect::<Vec<_>>();
        assert_eq!(kinds.len(), 6);
        assert_eq!(kinds.iter().filter(|k| **k == VerifiedInstructionKind::ClonePlace).count(), 1);
        assert_eq!(kinds.last(), Some(&VerifiedInstructionKind::VecConstruct));
        let clone = instructions
            .iter()
            .find(|i| i.kind() == VerifiedInstructionKind::ClonePlace)
            .expect("clone");
        assert_eq!(
            instructions.last().expect("Vec").value_operands().collect::<Vec<_>>(),
            [clone.result().expect("owned clone")]
        );
        assert_eq!(
            blocks[0].terminator().derived_drop_actions().count(),
            1,
            "original Pair survives clone and is dropped"
        );
        if let Some(previous) = &previous {
            assert_eq!(&kinds, previous);
        }
        previous = Some(kinds);
    }
}

#[test]
fn mixed_phase_valid_clone_summary_exact_extra_preserves_clone_context_and_ancestor_credits() {
    valid_source();
    let (source, snapshot) = phase_fixture(PhaseChild::AggregateClone, false);
    for extra in [false, true] {
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                let plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION - if extra { 3 } else { 4 };
                let at = span(lowerer.input.sources(), lowerer.function.span);
                // Retain real setup plans; unused suffix plans seed only the resource frontier.
                while lowerer.cleanup_plans.len() < plans {
                    let id = raw::CleanupPlanId(
                        u32::try_from(lowerer.cleanup_plans.len()).expect("bounded counter"),
                    );
                    lowerer.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions: vec![] });
                }
                lowerer.preparation_facts.held_cleanup = CleanupUsage {
                    plans,
                    actions: lowerer.cleanup_actions,
                    reserved_plans: 0,
                    reserved_actions: 0,
                }
                .reserve(2, OwnedCleanupReservationContext::Vec, at, lowerer.errors)
                .expect("checked external ancestor reservation");
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let clone_at = lowerer
                    .function
                    .body
                    .expressions
                    .iter()
                    .find(|e| matches!(e.kind, RawExpressionKind::Clone { .. }))
                    .expect("clone expression")
                    .span;
                let id = root_value(lowerer, 1);
                let prepared = PreparedValue::prepare(lowerer, id, ty);
                if extra {
                    assert!(prepared.is_none());
                    expected = Some(Diagnostic::error_at(
                        "ZRYNA-M3201",
                        span(lowerer.input.sources(), clone_at),
                        "structural clone exceeds a checked value, place, or cleanup resource limit",
                        "reduce simultaneously live owned aggregates or clone sites",
                    ));
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_facts, facts);
                } else {
                    let prepared = prepared.expect("exact clone frontier");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_facts, facts);
                    assert_eq!(prepared.plan.visits, 2);
                    let capacity = prepared
                        .plan
                        .steps
                        .iter()
                        .filter(|s| {
                            matches!(s.operation, Operation::CloneCapacity { aggregate: true })
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(capacity.len(), 1);
                    assert_eq!(capacity[0].after.held_cleanup, [2, 4]);
                    let result = prepared.plan.result;
                    assert_eq!(prepared.consume(), result);
                    assert_eq!(lowerer.cleanup_plans.len(), plans + 3);
                    assert_eq!(lowerer.preparation_facts.held_cleanup, [1, 2]);
                    assert_eq!(lowerer.owners.pending().len(), 2, "original Pair and returned Vec");
                    assert!(lowerer.constructor_storage_is_clear());
                    lowerer.preparation_facts.held_cleanup = CleanupUsage::release([1, 2], 2);
                }
            });
            assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
        }
    }
}
