use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, with_snapshot};
use super::*;
use crate::data_ownership_v1::owned_lowering_resources::{
    CleanupUsage, OwnedCleanupReservationContext,
};
use crate::data_ownership_v1::tests::{
    constructor_envelope_fixtures as fixtures, nested_mixed_construction::cleanup_frontier_fixture,
};
use crate::data_ownership_v1::{self as ownership, span};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{self as ir, VerifiedInstructionKind, VerifiedModule};
use zryna_syntax::v4::{RawExpressionKind, RawIdentifierSyntax, verify_snapshot};

const EXTERNAL_ACTIONS: usize = 3;

fn verify_source_control() {
    let (source, snapshot) = cleanup_frontier_fixture();
    let sources = fixtures::sources(&source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated empty-first source");
    let mut previous = None;
    for _ in 0..2 {
        let program = ownership::lower(fixtures::input(&syntax, &sources))
            .expect("unseeded source passes mandatory full IR verification");
        let functions = program.modules().flat_map(VerifiedModule::functions).collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let blocks = functions[0].blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 1);
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        let kinds = instructions.iter().map(|i| i.kind()).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::VecConstruct
            ]
        );
        assert_eq!(instructions[0].value_operands().count(), 0);
        assert_eq!(instructions[1].i32_literal(), Some(7));
        assert_eq!(
            instructions[2].value_operands().collect::<Vec<_>>(),
            [instructions[1].result().expect("literal")]
        );
        assert_eq!(
            instructions[3].value_operands().collect::<Vec<_>>(),
            [
                instructions[0].result().expect("empty Vec"),
                instructions[2].result().expect("filled Vec")
            ]
        );
        assert_eq!(functions[0].places().count(), 3);
        assert_eq!(blocks[0].terminator().derived_drop_actions().count(), 0);
        if let Some(previous) = &previous {
            assert_eq!(&kinds, previous);
        }
        previous = Some(kinds);
    }
}

pub(super) fn seed_external(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    plans: usize,
    actions: usize,
) {
    let at = span(lowerer.input.sources(), lowerer.function.span);
    // Authenticated source plus synthetic capacity counters, not a full-program frontier.
    // The external reservation itself is acquired through the actual checked shared API.
    lowerer.cleanup_plans = (0..plans)
        .map(|id| raw::CleanupPlan {
            id: raw::CleanupPlanId(u32::try_from(id).expect("bounded seed")),
            span: at,
            actions: vec![],
        })
        .collect();
    lowerer.cleanup_actions = actions;
    assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
    lowerer.preparation_facts.held_cleanup =
        CleanupUsage { plans, actions, reserved_plans: 0, reserved_actions: 0 }
            .reserve(EXTERNAL_ACTIONS, OwnedCleanupReservationContext::Vec, at, lowerer.errors)
            .expect("checked preexisting external reservation");
    assert_eq!(lowerer.preparation_facts.held_cleanup, [1, EXTERNAL_ACTIONS]);
}

fn check_success(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    ty: Ty,
    plans: usize,
    actions: usize,
) {
    let before = state(lowerer);
    let facts = lowerer.preparation_facts.clone();
    let id = root_value(lowerer, 0);
    let prepared = PreparedValue::prepare(lowerer, id, ty).expect("exact held cleanup frontier");
    assert_eq!(state(prepared.lowerer), before, "preparation never mutates real arenas/cache");
    assert_eq!(
        prepared.lowerer.preparation_facts, facts,
        "including real byte facts and held cleanup"
    );
    assert_eq!(prepared.plan.visits, 4);
    assert_eq!(prepared.plan.steps.len(), 13);
    // Actual recorded checkpoints: outer Enter, empty Enter/Release, filled Enter/Release,
    // outer Release. These constants assert the protocol; they do not simulate a planner.
    for (index, held) in
        [(0, [2, 5]), (1, [3, 5]), (2, [2, 5]), (5, [3, 6]), (7, [2, 5]), (10, [1, 3])]
    {
        assert_eq!(prepared.plan.steps[index].after.held_cleanup, held);
    }
    assert!(matches!(
        prepared.plan.steps[1].operation,
        Operation::Enter { arity: 0, kind: ConstructorKind::Vec, .. }
    ));
    for index in [2, 7, 10] {
        assert!(matches!(prepared.plan.steps[index].operation, Operation::Release));
    }
    let cleanup = prepared
        .plan
        .steps
        .iter()
        .filter_map(|step| match step.operation {
            Operation::Cleanup { actions, prefix: None, .. } => Some(actions),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(cleanup, [0, 1, 2]);
    assert_eq!(prepared.plan.facts.held_cleanup, [1, EXTERNAL_ACTIONS]);
    assert_eq!(prepared.consume(), raw::ValueId(3));
    assert_eq!(lowerer.cleanup_plans.len(), plans + 3);
    assert_eq!(lowerer.cleanup_actions, actions + 3);
    assert_eq!(lowerer.preparation_facts.held_cleanup, [1, EXTERNAL_ACTIONS]);
    assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(2)]);
    assert_eq!(lowerer.instructions.len(), 4);
    assert!(lowerer.constructor_storage_is_clear());
    lowerer.preparation_facts.held_cleanup =
        CleanupUsage::release(lowerer.preparation_facts.held_cleanup, EXTERNAL_ACTIONS);
    assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
}

fn frontier(plans: usize, actions: usize, rejected_expression: Option<usize>) {
    let (source, snapshot) = cleanup_frontier_fixture();
    for _ in 0..2 {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            seed_external(lowerer, plans, actions);
            if let Some(expression) = rejected_expression {
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(
                        lowerer.input.sources(),
                        lowerer.function.body.expressions[expression].span,
                    ),
                    "reserved Vec cleanup exceeds the per-function M3 limits",
                    "reduce simultaneously live owned values or fallible Vec operations",
                ));
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let id = root_value(lowerer, 0);
                assert!(PreparedValue::prepare(lowerer, id, ty).is_none());
                assert_eq!(state(lowerer), before, "all real state, including dense cache");
                assert_eq!(
                    lowerer.preparation_facts, facts,
                    "external credits and byte facts survive"
                );
            } else {
                check_success(lowerer, ty, plans, actions);
            }
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn mixed_cleanup_nested_plan_frontier_preserves_external_and_zero_action_child_credits() {
    verify_source_control();
    let max = ir::MAX_CLEANUP_PLANS_PER_FUNCTION;
    frontier(max - 4, 0, None);
    frontier(max - 3, 0, Some(2)); // First extra at filled sibling Enter.
    frontier(max - 2, 0, Some(0)); // Empty inner still requires its own plan, despite zero actions.
}

#[test]
fn mixed_cleanup_nested_action_frontier_releases_only_own_credits() {
    let max = ir::MAX_DROP_ACTIONS_PER_FUNCTION;
    frontier(0, max - 6, None);
    frontier(0, max - 5, Some(2));
}

#[test]
fn mixed_cleanup_summary_semantic_failure_precedes_exhausted_ancestor_capacity() {
    let (mut source, mut snapshot) = cleanup_frontier_fixture();
    let expression = &mut snapshot.files[0].functions[0].body.expressions[1];
    let at = expression.span;
    assert_eq!(&source[at.start as usize..at.end as usize], "7");
    source.replace_range(at.start as usize..at.end as usize, "x");
    expression.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "x".to_owned(), span: at },
    };
    for _ in 0..2 {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            seed_external(
                lowerer,
                ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
                ir::MAX_DROP_ACTIONS_PER_FUNCTION - EXTERNAL_ACTIONS,
            );
            expected = Some(Diagnostic::error_at(
                "ZRYNA-M3002",
                span(lowerer.input.sources(), at),
                "aggregate value 'x' is not declared",
                "reference one exact preceding local using its declared spelling",
            ));
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            let id = root_value(lowerer, 0);
            assert!(PreparedValue::prepare(lowerer, id, ty).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
        });
        assert_eq!(errors, [expected.expect("source-bound semantic diagnostic")]);
    }
}
