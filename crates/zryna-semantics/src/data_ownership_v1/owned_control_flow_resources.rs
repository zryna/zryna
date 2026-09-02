use std::collections::BTreeSet;

use super::diagnostics::Errors;
use super::global_resource_limits::resource_budget_violation;
use super::type_model::{
    RootBorrowArmPlan, RootBorrowBudgetLimit, RootBorrowInitializer, RootBorrowResources,
    RootBorrowStep,
};
use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

#[cfg(test)]
pub(super) fn straight_root_borrow_budget_violation(
    aliases: usize,
    reads: usize,
    writes: usize,
) -> Option<RootBorrowBudgetLimit> {
    root_borrow_resource_violation(RootBorrowResources {
        values: reads.saturating_add(writes).saturating_add(2),
        places: reads.saturating_add(1),
        transitions: aliases
            .saturating_mul(2)
            .saturating_add(reads.saturating_mul(2))
            .saturating_add(writes.saturating_mul(2))
            .saturating_add(3),
        blocks: 1,
        edges: 0,
        active_peak: aliases,
        cleanup_plans: 1,
    })
}

pub(super) fn root_borrow_resource_violation(
    resources: RootBorrowResources,
) -> Option<RootBorrowBudgetLimit> {
    if resources.values > ir::MAX_VALUES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Values)
    } else if resources.places > ir::MAX_PLACES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Places)
    } else if resources.transitions > ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Transitions)
    } else if resources.blocks > ir::MAX_BLOCKS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Blocks)
    } else if resources.edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Edges)
    } else if resources.active_peak > ir::MAX_ACTIVE_BORROWS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    } else if resources.cleanup_plans > ir::MAX_CLEANUP_PLANS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::CleanupPlans)
    } else {
        None
    }
}

pub(super) fn owned_root_borrow_resource_violation(
    existing_transitions: usize,
    lexical_owned_drops: usize,
    existing_active_borrows: usize,
) -> Option<RootBorrowBudgetLimit> {
    if existing_transitions
        .checked_add(lexical_owned_drops)
        .and_then(|total| total.checked_add(2))
        .is_none_or(|total| total > ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
    {
        Some(RootBorrowBudgetLimit::Transitions)
    } else if existing_active_borrows
        .checked_add(1)
        .is_none_or(|total| total > ir::MAX_ACTIVE_BORROWS_PER_FUNCTION)
    {
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    } else {
        None
    }
}

pub(super) fn conditional_root_borrow_budget_violation(
    then_aliases: usize,
    then_reads: usize,
    then_writes: usize,
    else_aliases: usize,
    else_reads: usize,
    else_writes: usize,
) -> Option<RootBorrowBudgetLimit> {
    root_borrow_resource_violation(conditional_root_borrow_resources(
        then_aliases,
        then_reads,
        then_writes,
        else_aliases,
        else_reads,
        else_writes,
    ))
}

pub(super) fn conditional_root_borrow_resources(
    then_aliases: usize,
    then_reads: usize,
    then_writes: usize,
    else_aliases: usize,
    else_reads: usize,
    else_writes: usize,
) -> RootBorrowResources {
    let aliases = then_aliases.saturating_add(else_aliases);
    let reads = then_reads.saturating_add(else_reads);
    let writes = then_writes.saturating_add(else_writes);
    RootBorrowResources {
        values: reads.saturating_add(writes).saturating_add(3),
        places: 1,
        transitions: aliases
            .saturating_mul(2)
            .saturating_add(reads)
            .saturating_add(writes.saturating_mul(2))
            .saturating_add(4),
        blocks: 4,
        edges: 4,
        active_peak: then_aliases.max(else_aliases),
        cleanup_plans: 1,
    }
}

pub(super) fn loop_root_borrow_resources(
    aliases: usize,
    reads: usize,
    writes: usize,
) -> RootBorrowResources {
    RootBorrowResources {
        values: reads.saturating_add(writes).saturating_add(3),
        places: 1,
        transitions: aliases
            .saturating_mul(2)
            .saturating_add(reads)
            .saturating_add(writes.saturating_mul(2))
            .saturating_add(4),
        blocks: 4,
        edges: 4,
        active_peak: aliases,
        cleanup_plans: 1,
    }
}

fn root_borrow_projection_place_count(steps: &[RootBorrowStep]) -> usize {
    let mut prefixes = BTreeSet::new();
    for place in steps.iter().filter_map(|step| match step {
        RootBorrowStep::Begin { place, .. } | RootBorrowStep::OwnerRead { place, .. } => {
            Some(place)
        }
        RootBorrowStep::Read { .. } | RootBorrowStep::Write { .. } => None,
    }) {
        let mut prefix = Vec::with_capacity(place.projections.len());
        for projection in &place.projections {
            prefix.push(projection.key);
            prefixes.insert(prefix.clone());
        }
    }
    prefixes.len()
}

fn root_borrow_write_value_count(steps: &[RootBorrowStep]) -> usize {
    steps
        .iter()
        .filter_map(|step| match step {
            RootBorrowStep::Write { value, .. } => Some(value.value_count()),
            _ => None,
        })
        .fold(0_usize, usize::saturating_add)
}

pub(super) fn projected_root_borrow_resources(
    initializer: &RootBorrowInitializer,
    arm: &RootBorrowArmPlan,
) -> RootBorrowResources {
    let write_values = root_borrow_write_value_count(&arm.steps);
    projected_root_borrow_resource_counts(
        initializer.value_count(),
        arm.aliases,
        arm.reads,
        arm.writes,
        write_values,
        root_borrow_projection_place_count(&arm.steps),
    )
}

pub(super) fn projected_root_borrow_resource_counts(
    initializer_values: usize,
    aliases: usize,
    reads: usize,
    writes: usize,
    write_values: usize,
    projection_places: usize,
) -> RootBorrowResources {
    RootBorrowResources {
        values: initializer_values
            .saturating_add(reads)
            .saturating_add(write_values)
            .saturating_add(1),
        places: reads.saturating_add(projection_places).saturating_add(1),
        transitions: initializer_values
            .saturating_add(1)
            .saturating_add(aliases.saturating_mul(2))
            .saturating_add(reads.saturating_mul(2))
            .saturating_add(write_values)
            .saturating_add(writes)
            .saturating_add(1),
        blocks: 1,
        edges: 0,
        active_peak: aliases,
        cleanup_plans: 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnedCfgBudgetLimit {
    Blocks,
    Edges,
    Transitions,
    Values,
}

pub(super) fn owned_cfg_budget_violation(
    blocks: usize,
    edges: usize,
    transitions: usize,
) -> Option<OwnedCfgBudgetLimit> {
    if blocks > ir::MAX_BLOCKS_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Blocks)
    } else if edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Edges)
    } else if transitions > ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
        Some(OwnedCfgBudgetLimit::Transitions)
    } else {
        None
    }
}

pub(super) fn dense_owned_value_id(count: usize) -> Option<raw::ValueId> {
    u32::try_from(count).ok().map(raw::ValueId)
}

pub(super) fn owned_value_budget_violation(current: usize, additional: usize) -> bool {
    resource_budget_violation(current, additional, ir::MAX_VALUES_PER_FUNCTION)
}

pub(super) fn owned_place_budget_violation(current: usize, additional: usize) -> bool {
    resource_budget_violation(current, additional, ir::MAX_PLACES_PER_FUNCTION)
}

pub(super) fn preflight_owned_place_capacity(
    current: usize,
    additional: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    if !owned_place_budget_violation(current, additional) {
        return true;
    }
    errors.at(
        "ZRYNA-M3201",
        at,
        format!(
            "derived places exceed the per-function M3 limit of {}",
            ir::MAX_PLACES_PER_FUNCTION
        ),
        "reduce owned parameters, expressions, and local declarations",
    );
    false
}

pub(super) fn preflight_owned_place_capacity_with_reserved(
    current: usize,
    reserved: usize,
    additional: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    let Some(committed_and_reserved) = current.checked_add(reserved) else {
        return preflight_owned_place_capacity(usize::MAX, 1, at, errors);
    };
    preflight_owned_place_capacity(committed_and_reserved, additional, at, errors)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EnumPayloadMoveResourceEstimate {
    pub(super) blocks: usize,
    pub(super) edges: usize,
    pub(super) values: usize,
    pub(super) places: usize,
    pub(super) transitions: usize,
    pub(super) cleanup_plans: usize,
    pub(super) cleanup_actions: usize,
}

pub(super) fn enum_payload_move_resource_estimate(
    payload_topology: usize,
) -> Option<EnumPayloadMoveResourceEstimate> {
    Some(EnumPayloadMoveResourceEstimate {
        blocks: 3,
        edges: 2,
        values: 3,
        places: payload_topology.checked_add(5)?,
        transitions: 4,
        cleanup_plans: 1,
        cleanup_actions: 0,
    })
}

pub(super) fn enum_payload_move_resource_violation(payload_topology: usize) -> bool {
    let Some(estimate) = enum_payload_move_resource_estimate(payload_topology) else {
        return true;
    };
    owned_cfg_budget_violation(estimate.blocks, estimate.edges, estimate.transitions).is_some()
        || owned_value_budget_violation(0, estimate.values)
        || owned_place_budget_violation(0, estimate.places)
        || resource_budget_violation(0, estimate.cleanup_plans, ir::MAX_CLEANUP_PLANS_PER_FUNCTION)
        || resource_budget_violation(0, estimate.cleanup_actions, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
}
