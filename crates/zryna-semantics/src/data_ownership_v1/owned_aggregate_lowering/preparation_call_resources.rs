use super::super::super::Errors;
use super::super::super::owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use super::super::super::owned_lowering_resources::{
    CleanupUsage, OwnedCleanupReservationContext, OwnedStringPreparationBudget,
    OwnedStringPreparationResources, preflight_owned_string_inputs,
};
use super::super::preparation_plan::{CallKind, Operation, PreparationPlan};
use super::super::preparation_state::Checkpoint;
use zryna_source::Span;

pub(super) fn enter(
    plan: &PreparationPlan<'_>,
    index: usize,
    before: Checkpoint,
    errors: &mut Errors<'_>,
) -> Option<(usize, [usize; 2])> {
    let step = &plan.steps[index];
    let Operation::CallEnter { signature, end, .. } = step.operation else {
        unreachable!("call entry");
    };
    let Operation::Cleanup { actions, prefix: None, .. } =
        plan.steps[end.checked_sub(2)?].operation
    else {
        unreachable!("call cleanup tail");
    };
    let after = plan.steps[if signature.kind == CallKind::String {
        end.checked_sub(1)?
    } else {
        end.checked_sub(3)?
    }]
    .after;
    let estimate = OwnedStringPreparationResources {
        values: after.counts[0].checked_sub(before.counts[0])?,
        places: after.counts[1].checked_sub(before.counts[1])?,
        transitions: after.counts[2].checked_sub(before.counts[2])?,
        cleanup_plans: after.counts[4].checked_sub(before.counts[4])?,
        cleanup_actions: after.counts[5].checked_sub(before.counts[5])?,
    };
    let own_cleanup = usize::from(signature.kind == CallKind::Vec);
    let budget = OwnedStringPreparationBudget {
        cleanup_plans: before.counts[4],
        cleanup_actions: before.counts[5],
        reserved_cleanup_plans: before.held_cleanup[0].checked_add(own_cleanup)?,
        reserved_cleanup_actions: before.held_cleanup[1].checked_add(if own_cleanup == 1 {
            actions
        } else {
            0
        })?,
        places: before.counts[1],
        reserved_places: before.held[3],
    };
    let capacity = super::resource_replay::vec_resources::capacity(before);
    if !preflight_owned_string_inputs(estimate, budget, &capacity, step.at, errors)
        || !capacity.transitions(estimate.transitions, step.at, errors)
        || !capacity.values(1, step.at, errors)
        || !preflight_owned_place_capacity_with_reserved(
            before.counts[1],
            before.held[3],
            1,
            step.at,
            errors,
        )
        || !capacity.transitions(1, step.at, errors)
    {
        return None;
    }
    let held = reserve(before, actions, signature.kind, step.at, errors)?;
    Some((actions, held))
}

pub(super) fn reserve(
    before: Checkpoint,
    actions: usize,
    kind: CallKind,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<[usize; 2]> {
    CleanupUsage {
        plans: before.counts[4],
        actions: before.counts[5],
        reserved_plans: before.held_cleanup[0],
        reserved_actions: before.held_cleanup[1],
    }
    .reserve(
        actions,
        if kind == CallKind::String {
            OwnedCleanupReservationContext::String
        } else {
            OwnedCleanupReservationContext::Vec
        },
        at,
        errors,
    )
}
