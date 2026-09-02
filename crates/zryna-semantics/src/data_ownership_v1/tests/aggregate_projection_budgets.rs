use super::*;

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
