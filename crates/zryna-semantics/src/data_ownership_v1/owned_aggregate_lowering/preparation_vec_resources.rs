use zryna_layout::{TypeCategory, VerifiedLayouts};
use zryna_source::Span;

use crate::data_ownership_v1::Errors;
use crate::data_ownership_v1::Ty;
use crate::data_ownership_v1::owned_aggregate_lowering::preparation_plan::{
    Operation, PreparationPlan,
};
use crate::data_ownership_v1::owned_aggregate_lowering::preparation_state::Checkpoint;
use crate::data_ownership_v1::owned_cfg_state::OwnedCfgCapacity;
use crate::data_ownership_v1::owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use crate::data_ownership_v1::owned_lowering_resources::{
    OwnedCleanupReservationContext, OwnedStringPreparationBudget, OwnedStringPreparationResources,
    preflight_owned_string_inputs,
};

pub(in crate::data_ownership_v1::owned_aggregate_lowering::constructor_preparation) fn capacity(
    before: Checkpoint,
) -> OwnedCfgCapacity {
    // This consumer is the existing single-block aggregate driver, not a synthetic CFG.
    OwnedCfgCapacity {
        values: before.counts[0],
        held_values: before.held[2],
        blocks: 1,
        edges: 0,
        transitions: before.counts[2],
        held_transitions: before.held[1],
    }
}

fn places(before: Checkpoint, additional: usize, at: Span, errors: &mut Errors<'_>) -> bool {
    preflight_owned_place_capacity_with_reserved(
        before.counts[1],
        before.held[3],
        additional,
        at,
        errors,
    )
}

fn sequence(before: Checkpoint, after: Checkpoint) -> Option<OwnedStringPreparationResources> {
    Some(OwnedStringPreparationResources {
        values: after.counts[0].checked_sub(before.counts[0])?,
        places: after.counts[1].checked_sub(before.counts[1])?,
        transitions: after.counts[2].checked_sub(before.counts[2])?,
        cleanup_plans: after.counts[4].checked_sub(before.counts[4])?,
        cleanup_actions: after.counts[5].checked_sub(before.counts[5])?,
    })
}

pub(super) fn enter(
    plan: &PreparationPlan<'_>,
    index: usize,
    before: Checkpoint,
    layouts: &VerifiedLayouts,
    errors: &mut Errors<'_>,
) -> Option<(usize, [usize; 2])> {
    let step = &plan.steps[index];
    let Operation::Enter { end, .. } = step.operation else { unreachable!("Vec entry") };
    let Operation::Cleanup { actions, prefix: None, .. } =
        plan.steps[end.checked_sub(2)?].operation
    else {
        unreachable!("Vec cleanup tail")
    };
    let element = layouts.type_by_id(step.ty.layout)?.referenced_type()?;
    if layouts.type_by_id(element)?.category() == TypeCategory::String {
        // Costs come from the already checked operation stream; this is not another syntax
        // estimator or ownership simulation. Only the fields consumed by the shared gate matter.
        let estimate = sequence(before, plan.steps[end.checked_sub(3)?].after)?;
        let budget = OwnedStringPreparationBudget {
            cleanup_plans: before.counts[4],
            cleanup_actions: before.counts[5],
            reserved_cleanup_plans: before.held_cleanup[0].checked_add(1)?,
            reserved_cleanup_actions: before.held_cleanup[1].checked_add(actions)?,
            places: before.counts[1],
            reserved_places: before.held[3],
        };
        if !preflight_owned_string_inputs(estimate, budget, &capacity(before), step.at, errors)
            || !capacity(before).transitions(estimate.transitions, step.at, errors)
        {
            return None;
        }
    }
    if !capacity(before).values(1, step.at, errors)
        || !places(before, 1, step.at, errors)
        || !capacity(before).transitions(1, step.at, errors)
    {
        return None;
    }
    let held = super::cleanup(before).reserve(
        actions,
        OwnedCleanupReservationContext::Vec,
        step.at,
        errors,
    )?;
    Some((actions, held))
}

pub(super) fn emit(before: Checkpoint, ty: Ty, at: Span, errors: &mut Errors<'_>) -> bool {
    (ty.is_copy() || places(before, 1, at, errors))
        && capacity(before).transitions(1, at, errors)
        && capacity(before).values(1, at, errors)
}
