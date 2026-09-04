use super::super::{OwnedCleanupAccounting, OwnerState, push_aggregate_clone_prefix_cleanup};
use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};

fn source() -> (SourceMap, Span) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".to_owned(),
        text: "cleanup".to_owned(),
    }])
    .expect("source map");
    let at = sources.verify_span(UntrustedSpan { file: 0, start: 1, end: 6 }).expect("span");
    (sources, at)
}

fn usage() -> CleanupUsage {
    CleanupUsage { plans: 0, actions: 0, reserved_plans: 0, reserved_actions: 0 }
}

fn owners() -> OwnerState {
    let mut owners = OwnerState::default();
    for index in 0..3 {
        owners.register_parameter(raw::PlaceId(index)).expect("unique owner");
    }
    owners
}

fn assert_error(errors: Errors<'_>, at: Span, message: &str, help: &'static str) {
    assert_eq!(errors.finish(), vec![Diagnostic::error_at("ZRYNA-M3201", at, message, help)]);
}

#[test]
fn cleanup_recipes_reverse_order_exclusion_and_contexts() {
    let (sources, at) = source();
    let owners = owners();
    for context in [
        OwnedCleanupPlanContext::String,
        OwnedCleanupPlanContext::Vec,
        OwnedCleanupPlanContext::Aggregate,
    ] {
        for (excluded, expected) in [
            (None, vec![2, 1, 0]),
            (Some(raw::PlaceId(1)), vec![2, 0]),
            (Some(raw::PlaceId(9)), vec![2, 1, 0]),
        ] {
            let mut errors = Errors::new(&sources);
            let recipe = CleanupRecipe::reverse(
                &usage(),
                owners.pending(),
                excluded,
                context,
                at,
                &mut errors,
            )
            .expect("recipe");
            assert_eq!(recipe.id, raw::CleanupPlanId(0));
            assert_eq!(recipe.action_count, expected.len());
            assert_eq!(
                recipe.into_actions().collect::<Vec<_>>(),
                expected
                    .into_iter()
                    .map(|index| raw::DropAction::DropPlace(raw::PlaceId(index)))
                    .collect::<Vec<_>>()
            );
            assert!(errors.finish().is_empty());
        }
    }
    assert_eq!(owners.pending(), [raw::PlaceId(0), raw::PlaceId(1), raw::PlaceId(2)]);
}

#[test]
fn cleanup_recipes_reverse_exact_and_first_extra_reserved_capacity() {
    let (sources, at) = source();
    let pending = [raw::PlaceId(0)];
    let exact = CleanupUsage {
        plans: ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        reserved_plans: 1,
        actions: ir::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
        reserved_actions: 1,
    };
    let mut errors = Errors::new(&sources);
    let recipe = CleanupRecipe::reverse(
        &exact,
        &pending,
        None,
        OwnedCleanupPlanContext::Aggregate,
        at,
        &mut errors,
    )
    .expect("exact frontier");
    assert_eq!(recipe.action_count, 1);
    assert_eq!(recipe.id, raw::CleanupPlanId(u32::try_from(exact.plans).expect("bounded ID")));
    assert!(errors.finish().is_empty());
    for plans_first in [true, false] {
        let extra = CleanupUsage {
            reserved_plans: if plans_first { 2 } else { 1 },
            reserved_actions: 2,
            ..exact
        };
        let mut errors = Errors::new(&sources);
        assert!(
            CleanupRecipe::reverse(
                &extra,
                &pending,
                None,
                OwnedCleanupPlanContext::Aggregate,
                at,
                &mut errors
            )
            .is_none()
        );
        let (dimension, limit, help) = if plans_first {
            (
                "sites",
                ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
                "reduce fallible String leaves in private aggregate construction",
            )
        } else {
            (
                "actions",
                ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                "reduce simultaneously live owned aggregates and String leaves",
            )
        };
        assert_error(
            errors,
            at,
            &format!("derived cleanup {dimension} exceed the per-function M3 limit of {limit}"),
            help,
        );
    }
}

#[test]
fn cleanup_recipes_reverse_context_diagnostics_and_zero_cost_overflow() {
    let (sources, at) = source();
    for (context, plan_help, action_help) in [
        (
            OwnedCleanupPlanContext::String,
            "reduce fallible private String operations",
            "reduce simultaneously live Strings or fallible private String operations",
        ),
        (
            OwnedCleanupPlanContext::Vec,
            "reduce fallible private Vec operations",
            "reduce simultaneously live owned values or fallible private Vec operations",
        ),
        (
            OwnedCleanupPlanContext::Aggregate,
            "reduce fallible String leaves in private aggregate construction",
            "reduce simultaneously live owned aggregates and String leaves",
        ),
    ] {
        for (budget, dimension, limit, help) in [
            (
                CleanupUsage {
                    reserved_plans: usize::MAX,
                    reserved_actions: usize::MAX,
                    ..usage()
                },
                "sites",
                ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
                plan_help,
            ),
            (
                CleanupUsage { actions: usize::MAX, ..usage() },
                "actions",
                ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                action_help,
            ),
            (
                CleanupUsage { reserved_actions: usize::MAX, ..usage() },
                "actions",
                ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                action_help,
            ),
        ] {
            let mut errors = Errors::new(&sources);
            assert!(CleanupRecipe::reverse(&budget, &[], None, context, at, &mut errors).is_none());
            assert_error(
                errors,
                at,
                &format!("derived cleanup {dimension} exceed the per-function M3 limit of {limit}"),
                help,
            );
        }
        let mut errors = Errors::new(&sources);
        let recipe = CleanupRecipe::reverse(&usage(), &[], None, context, at, &mut errors)
            .expect("valid replay");
        assert_eq!(recipe.action_count, 0);
        assert_eq!(recipe.into_actions().count(), 0);
        assert!(errors.finish().is_empty());
    }
}

#[test]
fn cleanup_recipes_vec_prefix_order_and_reserved_frontiers() {
    let (sources, at) = source();
    let owners = owners();
    let exact = CleanupUsage {
        plans: ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        reserved_plans: 1,
        actions: ir::MAX_DROP_ACTIONS_PER_FUNCTION - 5,
        reserved_actions: 1,
    };
    let mut errors = Errors::new(&sources);
    let recipe =
        CleanupRecipe::vec_prefix(&exact, owners.pending(), raw::PlaceId(3), at, &mut errors)
            .expect("exact prefix frontier");
    assert_eq!(recipe.action_count, 4);
    assert_eq!(
        recipe.into_actions().collect::<Vec<_>>(),
        [
            raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(3)),
            raw::DropAction::DropPlace(raw::PlaceId(2)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ]
    );
    assert!(errors.finish().is_empty());
    for budget in [
        CleanupUsage { reserved_plans: 2, ..exact },
        CleanupUsage { reserved_actions: 2, ..exact },
        CleanupUsage { reserved_plans: usize::MAX, ..exact },
        CleanupUsage { reserved_actions: usize::MAX, ..exact },
        CleanupUsage { actions: usize::MAX, ..exact },
    ] {
        let mut errors = Errors::new(&sources);
        assert!(
            CleanupRecipe::vec_prefix(&budget, owners.pending(), raw::PlaceId(3), at, &mut errors)
                .is_none()
        );
        assert_error(
            errors,
            at,
            "Vec clone element cleanup exceeds the per-function M3 limits",
            "reduce simultaneously live owned values or fallible Vec clones",
        );
    }
}

#[test]
fn cleanup_recipes_checked_vec_count_is_helper_only_overflow_evidence() {
    let (sources, at) = source();
    let mut errors = Errors::new(&sources);
    // This synthetic count is not a reachable pending-owner slice or full verifier proof.
    assert_eq!(checked_vec_clone_prefix_action_count(usize::MAX, at, &mut errors), None);
    assert_error(
        errors,
        at,
        "Vec clone prefix cleanup overflows its checked action count",
        "reduce simultaneously live owned values",
    );
    let mut errors = Errors::new(&sources);
    assert_eq!(checked_vec_clone_prefix_action_count(0, at, &mut errors), Some(1));
    assert!(errors.finish().is_empty());
}

#[test]
fn cleanup_recipes_aggregate_prefix_retains_caller_capacity_contract() {
    let owners = owners();
    // Synthetic usage isolates the absence of a new capacity gate, not full IR admission.
    let plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION + 1;
    let recipe = CleanupRecipe::aggregate_prefix(plans, owners.pending(), raw::PlaceId(3))
        .expect("caller establishes capacity");
    assert_eq!(recipe.id, raw::CleanupPlanId(u32::try_from(plans).expect("bounded ID")));
    assert_eq!(recipe.action_count, 4);
    assert_eq!(
        recipe.into_actions().collect::<Vec<_>>(),
        [
            raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(3)),
            raw::DropAction::DropPlace(raw::PlaceId(2)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ]
    );
    if let Some(invalid_id) = usize::try_from(u32::MAX).expect("u32 fits usize").checked_add(1) {
        assert!(CleanupRecipe::aggregate_prefix(invalid_id, &[], raw::PlaceId(3)).is_none());
    }
}

#[test]
fn cleanup_recipes_live_accounting_preserves_commit_and_reservations() {
    let (sources, at) = source();
    let owners = owners();
    let mut plans = Vec::new();
    let (mut actions, mut held_plans, mut held_actions) = (0, 1, 2);
    let mut errors = Errors::new(&sources);
    {
        let mut accounting = OwnedCleanupAccounting::new(
            &mut plans,
            &mut actions,
            &mut held_plans,
            &mut held_actions,
        );
        assert_eq!(
            accounting.push_reverse(
                &owners,
                at,
                Some(raw::PlaceId(1)),
                OwnedCleanupPlanContext::String,
                &mut errors
            ),
            Some(raw::CleanupPlanId(0))
        );
        assert_eq!(
            accounting.push_vec_clone_prefix(&owners, raw::PlaceId(3), at, &mut errors),
            Some(raw::CleanupPlanId(1))
        );
    }
    assert_eq!((actions, held_plans, held_actions), (6, 1, 2));
    assert_eq!(
        plans[0].actions,
        [raw::DropAction::DropPlace(raw::PlaceId(2)), raw::DropAction::DropPlace(raw::PlaceId(0))]
    );
    assert_eq!(plans[1].actions[0], raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(3)));
    assert_eq!(
        push_aggregate_clone_prefix_cleanup(&mut plans, &mut actions, &owners, raw::PlaceId(3), at),
        Some(raw::CleanupPlanId(2))
    );
    assert_eq!((actions, held_plans, held_actions), (10, 1, 2));
    assert_eq!(
        plans[2].actions[0],
        raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(3))
    );
    for (index, plan) in plans.iter().enumerate() {
        assert_eq!(plan.id, raw::CleanupPlanId(u32::try_from(index).expect("small ID")));
        assert_eq!(plan.span, at);
    }
    assert!(errors.finish().is_empty());
}

#[test]
fn cleanup_recipes_live_failure_leaves_accounting_unchanged_then_replays() {
    let (sources, at) = source();
    let owners = owners();
    let mut plans = Vec::new();
    let (mut actions, mut held_plans, mut held_actions) = (0, 0, usize::MAX);
    let mut errors = Errors::new(&sources);
    assert_eq!(
        OwnedCleanupAccounting::new(&mut plans, &mut actions, &mut held_plans, &mut held_actions)
            .push_reverse(&owners, at, None, OwnedCleanupPlanContext::Vec, &mut errors),
        None
    );
    assert!(plans.is_empty());
    assert_eq!((actions, held_plans, held_actions), (0, 0, usize::MAX));
    assert_error(
        errors,
        at,
        &format!(
            "derived cleanup actions exceed the per-function M3 limit of {}",
            ir::MAX_DROP_ACTIONS_PER_FUNCTION
        ),
        "reduce simultaneously live owned values or fallible private Vec operations",
    );
    held_actions = 0;
    let mut errors = Errors::new(&sources);
    assert_eq!(
        OwnedCleanupAccounting::new(&mut plans, &mut actions, &mut held_plans, &mut held_actions)
            .push_reverse(&owners, at, None, OwnedCleanupPlanContext::Vec, &mut errors),
        Some(raw::CleanupPlanId(0))
    );
    assert_eq!((plans.len(), actions, held_plans, held_actions), (1, 3, 0, 0));
    assert_eq!(owners.pending(), [raw::PlaceId(0), raw::PlaceId(1), raw::PlaceId(2)]);
    assert!(errors.finish().is_empty());
}
