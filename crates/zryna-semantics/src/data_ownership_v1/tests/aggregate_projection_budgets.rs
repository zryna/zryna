use super::*;

fn assert_aggregate_cleanup_authority_order() {
    let sources = sources_for(OWNED_PAIR_SOURCE);
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 0, end: 1 });
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0), raw::PlaceId(1), raw::PlaceId(2)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let mut plans = Vec::new();
    let mut committed_actions = 0;
    let mut errors = Errors::new(&sources);
    assert_eq!(
        push_aggregate_reverse_cleanup(
            &mut plans,
            &mut committed_actions,
            &owners,
            at,
            Some(raw::PlaceId(1)),
            &mut errors,
        ),
        Some(raw::CleanupPlanId(0)),
    );
    assert_eq!(plans[0].span, at);
    assert_eq!(
        plans[0].actions,
        [raw::DropAction::DropPlace(raw::PlaceId(2)), raw::DropAction::DropPlace(raw::PlaceId(0))],
    );
    assert_eq!(committed_actions, 2);

    assert_eq!(
        push_aggregate_clone_prefix_cleanup(
            &mut plans,
            &mut committed_actions,
            &owners,
            raw::PlaceId(3),
            at,
        ),
        Some(raw::CleanupPlanId(1)),
    );
    assert_eq!(plans[1].span, at);
    assert_eq!(
        plans[1].actions,
        [
            raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(3)),
            raw::DropAction::DropPlace(raw::PlaceId(2)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
    );
    assert_eq!(committed_actions, 6);
    assert!(errors.finish().is_empty());
}

fn assert_aggregate_cleanup_diagnostic(
    diagnostic: &zryna_diagnostics::Diagnostic,
    at: zryna_source::Span,
    message: &str,
    guidance: &str,
) {
    assert_eq!(diagnostic.code(), "ZRYNA-M3201");
    assert_eq!(diagnostic.primary_span(), Some(at));
    assert_eq!(diagnostic.message(), message);
    assert_eq!(diagnostic.guidance(), guidance);
}

fn assert_aggregate_cleanup_authority_diagnostics() {
    let sources = sources_for(OWNED_PAIR_SOURCE);
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 0, end: 1 });
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let full_plan = raw::CleanupPlan { id: raw::CleanupPlanId(0), span: at, actions: Vec::new() };
    let mut full_plans =
        vec![full_plan; zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION];
    let mut committed_actions = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    let mut plan_errors = Errors::new(&sources);
    assert_eq!(
        push_aggregate_reverse_cleanup(
            &mut full_plans,
            &mut committed_actions,
            &owners,
            at,
            None,
            &mut plan_errors,
        ),
        None,
    );
    assert_eq!(full_plans.len(), zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION,);
    assert_eq!(committed_actions, zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION);
    let diagnostics = plan_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_aggregate_cleanup_diagnostic(
        &diagnostics[0],
        at,
        &format!(
            "derived cleanup sites exceed the per-function M3 limit of {}",
            zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ),
        "reduce fallible String leaves in private aggregate construction",
    );

    let mut plans = Vec::new();
    let mut action_errors = Errors::new(&sources);
    assert_eq!(
        push_aggregate_reverse_cleanup(
            &mut plans,
            &mut committed_actions,
            &owners,
            at,
            None,
            &mut action_errors,
        ),
        None,
    );
    assert!(plans.is_empty());
    assert_eq!(committed_actions, zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION);
    let diagnostics = action_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_aggregate_cleanup_diagnostic(
        &diagnostics[0],
        at,
        &format!(
            "derived cleanup actions exceed the per-function M3 limit of {}",
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION,
        ),
        "reduce simultaneously live owned aggregates and String leaves",
    );
}

#[test]
fn structural_clone_resource_preflight_accepts_exact_limits_and_rejects_excess_or_overflow() {
    assert!(!aggregate_clone_budget_violation(
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 3,
        1,
    ));
    assert!(aggregate_clone_budget_violation(
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
        0,
        0,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
        0,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
        0,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
        0,
        0,
    ));
    assert!(aggregate_clone_budget_violation(
        0,
        0,
        0,
        0,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
        1,
    ));
    assert!(aggregate_clone_budget_violation(0, 0, 0, 0, usize::MAX, 0));
    assert!(aggregate_clone_budget_violation(0, 0, 0, 0, 0, usize::MAX));
    assert_aggregate_cleanup_authority_order();
    assert_aggregate_cleanup_authority_diagnostics();
}
#[test]
fn projected_aggregate_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 4,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        3,
    ));
    for (values, places, transitions, reserved, missing) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 3, 0, 0, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0),
        (0, usize::MAX, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX),
    ] {
        assert!(projected_aggregate_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            missing,
        ));
    }
}
#[test]
fn projected_subobject_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_subobject_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 10,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        2,
        3,
        4,
    ));
    for (values, places, transitions, reserved, source_path, descendants, target_path) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 9, 0, 0, 2, 3, 4),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX, 0, 0),
        (0, 0, 0, 0, 0, usize::MAX, 0),
        (0, 0, 0, 0, 0, 0, usize::MAX),
    ] {
        assert!(projected_subobject_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            source_path,
            descendants,
            target_path,
        ));
    }
}
#[test]
fn projected_aggregate_clone_assignment_resource_preflight_is_exact_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_clone_assignment_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 6,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        MAX_DROP_ACTIONS_PER_FUNCTION - 5,
        2,
        2,
        3,
    ));
    for (values, places, transitions, reserved, plans, actions, pending, source, target) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 5, 0, 0, 0, 0, 0, 2, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION - 1, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 4, 2, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, 0, 0, usize::MAX, 0),
        (0, 0, 0, 0, 0, 0, 0, 0, usize::MAX),
        (0, 0, 0, 0, 0, 0, usize::MAX, 0, 0),
    ] {
        assert!(projected_aggregate_clone_assignment_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
            source,
            target,
        ));
    }
}
#[test]
fn projected_aggregate_clone_resource_preflight_is_exact_plus_one_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_aggregate_clone_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 5,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 3,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
        MAX_DROP_ACTIONS_PER_FUNCTION - 5,
        2,
        3,
    ));
    for (values, places, transitions, reserved, plans, actions, pending, missing) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION - 4, 0, 0, 0, 0, 0, 3),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2, 1, 0, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION - 1, 0, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 4, 2, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0, 0),
        (0, 0, 0, 0, 0, 0, 0, usize::MAX),
        (0, 0, 0, 0, 0, 0, usize::MAX, 0),
    ] {
        assert!(projected_aggregate_clone_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
            missing,
        ));
    }
}
#[test]
fn projected_string_clone_resource_preflight_is_exact_plus_one_and_overflow_checked() {
    use zryna_ir::data_ownership_v1::{
        MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DROP_ACTIONS_PER_FUNCTION,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION, MAX_VALUES_PER_FUNCTION,
    };

    assert!(!projected_string_clone_budget_violation(
        MAX_VALUES_PER_FUNCTION - 1,
        MAX_PLACES_PER_FUNCTION - 1,
        MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2,
        1,
        MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
        MAX_DROP_ACTIONS_PER_FUNCTION - 2,
        2,
    ));
    for (values, places, transitions, reserved, plans, actions, pending) in [
        (MAX_VALUES_PER_FUNCTION, 0, 0, 0, 0, 0, 0),
        (0, MAX_PLACES_PER_FUNCTION, 0, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, 0, 0, 0, 0),
        (0, 0, MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1, 1, 0, 0, 0),
        (0, 0, 0, 0, MAX_CLEANUP_PLANS_PER_FUNCTION, 0, 0),
        (0, 0, 0, 0, 0, MAX_DROP_ACTIONS_PER_FUNCTION - 1, 2),
        (usize::MAX, 0, 0, 0, 0, 0, 0),
        (0, usize::MAX, 0, 0, 0, 0, 0),
        (0, 0, usize::MAX, 0, 0, 0, 0),
        (0, 0, 0, usize::MAX, 0, 0, 0),
        (0, 0, 0, 0, usize::MAX, 0, 0),
        (0, 0, 0, 0, 0, usize::MAX, 1),
        (0, 0, 0, 0, 0, 1, usize::MAX),
    ] {
        assert!(projected_string_clone_budget_violation(
            values,
            places,
            transitions,
            reserved,
            plans,
            actions,
            pending,
        ));
    }
}
