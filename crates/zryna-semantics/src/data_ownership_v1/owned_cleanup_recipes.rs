use super::{
    Errors, OwnedCleanupPlanContext, checked_vec_clone_prefix_action_count,
    resource_budget_violation,
};
use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

#[cfg(test)]
#[path = "tests/owned_cleanup_recipes.rs"]
mod tests;

pub(in crate::data_ownership_v1) struct CleanupUsage {
    pub(in crate::data_ownership_v1) plans: usize,
    pub(in crate::data_ownership_v1) actions: usize,
    pub(in crate::data_ownership_v1) reserved_plans: usize,
    pub(in crate::data_ownership_v1) reserved_actions: usize,
}

// The pending sequence comes from OwnerState's unique-owner invariant. This description
// preserves existing counts and action order; it neither verifies owners nor issues capacity.
pub(in crate::data_ownership_v1) struct CleanupRecipe<'a> {
    pub(in crate::data_ownership_v1) id: raw::CleanupPlanId,
    pub(in crate::data_ownership_v1) action_count: usize,
    pending: &'a [raw::PlaceId],
    excluded: Option<raw::PlaceId>,
    prefix: Option<raw::DropAction>,
}

impl<'a> CleanupRecipe<'a> {
    pub(in crate::data_ownership_v1) fn reverse(
        usage: &CleanupUsage,
        pending: &'a [raw::PlaceId],
        excluded: Option<raw::PlaceId>,
        context: OwnedCleanupPlanContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Self> {
        if resource_budget_violation(
            usage.plans,
            usage.reserved_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup sites exceed the per-function M3 limit of {}",
                    ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                ),
                context.plan_guidance(),
            );
            return None;
        }
        let excluded_present = excluded.is_some_and(|place| pending.contains(&place));
        let action_count = pending.len() - usize::from(excluded_present);
        if resource_budget_violation(
            usage.actions,
            usage.reserved_actions.saturating_add(action_count),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                context.action_guidance(),
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(usage.plans).unwrap_or(u32::MAX));
        Some(Self { id, action_count, pending, excluded, prefix: None })
    }

    pub(super) fn vec_prefix(
        usage: &CleanupUsage,
        pending: &'a [raw::PlaceId],
        result_owner: raw::PlaceId,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Self> {
        let action_count = checked_vec_clone_prefix_action_count(pending.len(), at, errors)?;
        if resource_budget_violation(
            usage.plans,
            usage.reserved_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            usage.actions,
            usage.reserved_actions.saturating_add(action_count),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                "Vec clone element cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live owned values or fallible Vec clones",
            );
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(usage.plans).ok()?);
        Some(Self {
            id,
            action_count,
            pending,
            excluded: None,
            prefix: Some(raw::DropAction::DropVecInitializedPrefix(result_owner)),
        })
    }

    // Aggregate clone callers already establish capacity. This retains only the original
    // checked count and plan-ID conversion, not a new independent budget gate.
    pub(in crate::data_ownership_v1) fn aggregate_prefix(
        plans: usize,
        pending: &'a [raw::PlaceId],
        result_owner: raw::PlaceId,
    ) -> Option<Self> {
        let action_count = pending.len().checked_add(1)?;
        let id = raw::CleanupPlanId(u32::try_from(plans).ok()?);
        Some(Self {
            id,
            action_count,
            pending,
            excluded: None,
            prefix: Some(raw::DropAction::DropAggregateInitializedPrefix(result_owner)),
        })
    }

    pub(in crate::data_ownership_v1) fn into_actions(
        self,
    ) -> impl Iterator<Item = raw::DropAction> + 'a {
        let Self { pending, excluded, prefix, .. } = self;
        prefix.into_iter().chain(
            pending
                .iter()
                .rev()
                .copied()
                .filter(move |place| Some(*place) != excluded)
                .map(raw::DropAction::DropPlace),
        )
    }
}
