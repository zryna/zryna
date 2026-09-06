use super::*;

use zryna_ir::data_ownership_v1 as ir;

fn only(dimension: RootBorrowBudgetLimit, count: usize) -> RootBorrowResources {
    let mut resources = RootBorrowResources::default();
    match dimension {
        RootBorrowBudgetLimit::Values => resources.values = count,
        RootBorrowBudgetLimit::Places => resources.places = count,
        RootBorrowBudgetLimit::Transitions => resources.transitions = count,
        RootBorrowBudgetLimit::Blocks => resources.blocks = count,
        RootBorrowBudgetLimit::Edges => resources.edges = count,
        RootBorrowBudgetLimit::ActiveBorrows => resources.active_peak = count,
        RootBorrowBudgetLimit::CleanupPlans => resources.cleanup_plans = count,
    }
    resources
}

#[test]
fn nonindexed_owned_borrow_resource_dimensions_accept_exact_and_reject_first_extra() {
    for (dimension, limit) in [
        (RootBorrowBudgetLimit::Places, ir::MAX_PLACES_PER_FUNCTION),
        (RootBorrowBudgetLimit::Transitions, ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION),
        (RootBorrowBudgetLimit::Blocks, ir::MAX_BLOCKS_PER_FUNCTION),
        (RootBorrowBudgetLimit::Edges, ir::MAX_CFG_EDGES_PER_FUNCTION),
        (RootBorrowBudgetLimit::ActiveBorrows, ir::MAX_ACTIVE_BORROWS_PER_FUNCTION),
        (RootBorrowBudgetLimit::CleanupPlans, ir::MAX_CLEANUP_PLANS_PER_FUNCTION),
    ] {
        assert_eq!(root_borrow_resource_violation(only(dimension, limit)), None);
        assert_eq!(root_borrow_resource_violation(only(dimension, limit + 1)), Some(dimension));
    }
    assert!(!resource_budget_violation(
        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        0,
        ir::MAX_DROP_ACTIONS_PER_FUNCTION
    ));
    assert!(resource_budget_violation(
        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        1,
        ir::MAX_DROP_ACTIONS_PER_FUNCTION
    ));
}

#[test]
fn nonindexed_owned_borrow_resource_overflow_is_checked_and_recovery_is_stable() {
    let hostile = projected_root_borrow_resource_counts(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    );
    assert_eq!(hostile.values, usize::MAX);
    assert_eq!(hostile.places, usize::MAX);
    assert_eq!(hostile.transitions, usize::MAX);
    assert_eq!(hostile.active_peak, usize::MAX);

    let reject = || root_borrow_resource_violation(hostile);
    assert_eq!(reject(), Some(RootBorrowBudgetLimit::Values));
    assert_eq!(reject(), Some(RootBorrowBudgetLimit::Values));
    assert!(resource_budget_violation(usize::MAX, 1, ir::MAX_PLACES_PER_FUNCTION));
    assert!(resource_budget_violation(
        usize::MAX,
        usize::MAX,
        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
    ));

    let valid = projected_root_borrow_resource_counts(1, 1, 0, 0, 0, 2);
    assert_eq!(valid.places, 3);
    assert_eq!(valid.transitions, 5);
    assert_eq!(valid.active_peak, 1);
    assert_eq!(root_borrow_resource_violation(valid), None);
}
