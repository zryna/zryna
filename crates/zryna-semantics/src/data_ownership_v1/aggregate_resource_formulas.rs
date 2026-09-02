use super::global_resource_limits::{
    aggregate_transition_budget_violation, resource_budget_violation,
};
use super::owned_control_flow_resources::{
    owned_place_budget_violation, owned_value_budget_violation,
};
use zryna_ir::data_ownership_v1 as ir;

pub(super) fn partial_transfer_place_delta(topology: usize, existing: usize) -> Option<usize> {
    if existing > topology {
        return None;
    }
    topology.checked_mul(3)?.checked_sub(existing)?.checked_add(2)
}

pub(super) fn partial_return_place_delta(topology: usize, existing: usize) -> Option<usize> {
    if existing > topology {
        return None;
    }
    topology.checked_mul(2)?.checked_sub(existing)?.checked_add(1)
}

pub(super) fn partial_assignment_place_delta(
    topology: usize,
    source_existing: usize,
    target_existing: usize,
) -> Option<usize> {
    if source_existing > topology || target_existing > topology {
        return None;
    }
    topology
        .checked_mul(3)?
        .checked_sub(source_existing)?
        .checked_sub(target_existing)?
        .checked_add(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PartialTransferBudgetViolation {
    PlaceAccounting,
    Values,
    Places,
    Transitions,
}

pub(super) fn partial_transfer_budget_preflight(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    topology: usize,
    existing: usize,
) -> Result<usize, PartialTransferBudgetViolation> {
    let additional_places = partial_transfer_place_delta(topology, existing)
        .ok_or(PartialTransferBudgetViolation::PlaceAccounting)?;
    if owned_value_budget_violation(values, 1) {
        return Err(PartialTransferBudgetViolation::Values);
    }
    if owned_place_budget_violation(places, additional_places) {
        return Err(PartialTransferBudgetViolation::Places);
    }
    if aggregate_transition_budget_violation(transitions, reserved_transitions, 2) {
        return Err(PartialTransferBudgetViolation::Transitions);
    }
    Ok(additional_places)
}

pub(super) fn partial_return_budget_preflight(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    topology: usize,
    existing: usize,
) -> Result<usize, PartialTransferBudgetViolation> {
    let additional_places = partial_return_place_delta(topology, existing)
        .ok_or(PartialTransferBudgetViolation::PlaceAccounting)?;
    if owned_value_budget_violation(values, 1) {
        return Err(PartialTransferBudgetViolation::Values);
    }
    if owned_place_budget_violation(places, additional_places) {
        return Err(PartialTransferBudgetViolation::Places);
    }
    if aggregate_transition_budget_violation(transitions, reserved_transitions, 1) {
        return Err(PartialTransferBudgetViolation::Transitions);
    }
    Ok(additional_places)
}

pub(super) fn partial_assignment_budget_preflight(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    topology: usize,
    source_existing: usize,
    target_existing: usize,
) -> Result<usize, PartialTransferBudgetViolation> {
    let additional_places =
        partial_assignment_place_delta(topology, source_existing, target_existing)
            .ok_or(PartialTransferBudgetViolation::PlaceAccounting)?;
    if owned_value_budget_violation(values, 1) {
        return Err(PartialTransferBudgetViolation::Values);
    }
    if owned_place_budget_violation(places, additional_places) {
        return Err(PartialTransferBudgetViolation::Places);
    }
    if aggregate_transition_budget_violation(transitions, reserved_transitions, 2) {
        return Err(PartialTransferBudgetViolation::Transitions);
    }
    Ok(additional_places)
}

pub(super) fn projected_subobject_move_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    missing_descendants: usize,
) -> bool {
    let Some(additional_places) = missing_descendants.checked_add(1) else { return true };
    owned_value_budget_violation(values, 1)
        || owned_place_budget_violation(places, additional_places)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 1)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projected_subobject_return_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
    missing_path: usize,
    missing_descendants: usize,
) -> bool {
    let Some(additional_places) =
        missing_path.checked_add(missing_descendants).and_then(|count| count.checked_add(1))
    else {
        return true;
    };
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, additional_places, ir::MAX_PLACES_PER_FUNCTION)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 1)
        || resource_budget_violation(cleanup_plans, 1, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(cleanup_actions, pending, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
}

pub(super) fn projected_aggregate_assignment_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    missing_path_places: usize,
) -> bool {
    let Some(additional_places) = missing_path_places.checked_add(1) else { return true };
    owned_value_budget_violation(values, 1)
        || owned_place_budget_violation(places, additional_places)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 2)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projected_subobject_assignment_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    source_missing_path_places: usize,
    source_missing_descendant_places: usize,
    target_missing_path_places: usize,
) -> bool {
    let Some(additional_places) = source_missing_path_places
        .checked_add(source_missing_descendant_places)
        .and_then(|total| total.checked_add(target_missing_path_places))
        .and_then(|total| total.checked_add(1))
    else {
        return true;
    };
    owned_value_budget_violation(values, 1)
        || owned_place_budget_violation(places, additional_places)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 2)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projected_aggregate_clone_assignment_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
    source_missing_path_places: usize,
    target_missing_path_places: usize,
) -> bool {
    let Some(additional_places) = source_missing_path_places
        .checked_add(target_missing_path_places)
        .and_then(|total| total.checked_add(1))
    else {
        return true;
    };
    let Some(prefix_actions) = pending.checked_add(1) else { return true };
    let Some(additional_actions) = pending.checked_add(prefix_actions) else { return true };
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, additional_places, ir::MAX_PLACES_PER_FUNCTION)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 2)
        || resource_budget_violation(cleanup_plans, 2, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(
            cleanup_actions,
            additional_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
}

pub(super) fn aggregate_clone_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
) -> bool {
    let Some(prefix_actions) = pending.checked_add(1) else { return true };
    let Some(new_actions) = pending.checked_add(prefix_actions) else { return true };
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, 1, ir::MAX_PLACES_PER_FUNCTION)
        || resource_budget_violation(transitions, 1, ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
        || resource_budget_violation(cleanup_plans, 2, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(
            cleanup_actions,
            new_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projected_aggregate_clone_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
    missing_path_places: usize,
) -> bool {
    let Some(additional_places) = missing_path_places.checked_add(2) else { return true };
    let Some(prefix_actions) = pending.checked_add(1) else { return true };
    let Some(additional_actions) = pending.checked_add(prefix_actions) else { return true };
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, additional_places, ir::MAX_PLACES_PER_FUNCTION)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 2)
        || resource_budget_violation(cleanup_plans, 2, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(
            cleanup_actions,
            additional_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
}

pub(super) fn projected_string_clone_budget_violation(
    values: usize,
    places: usize,
    transitions: usize,
    reserved_transitions: usize,
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
) -> bool {
    resource_budget_violation(values, 1, ir::MAX_VALUES_PER_FUNCTION)
        || resource_budget_violation(places, 1, ir::MAX_PLACES_PER_FUNCTION)
        || aggregate_transition_budget_violation(transitions, reserved_transitions, 1)
        || resource_budget_violation(cleanup_plans, 1, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || cleanup_action_budget_violation(cleanup_actions, pending, false)
}

pub(super) fn cleanup_action_budget_violation(
    current: usize,
    pending: usize,
    excluded_present: bool,
) -> bool {
    let actions = pending.saturating_sub(usize::from(excluded_present));
    resource_budget_violation(current, actions, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
}
