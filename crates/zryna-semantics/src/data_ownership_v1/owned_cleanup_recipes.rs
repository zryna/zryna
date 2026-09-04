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

impl CleanupUsage {
    pub(in crate::data_ownership_v1) fn reserve(
        &self,
        actions: usize,
        context: super::OwnedCleanupReservationContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<[usize; 2]> {
        if resource_budget_violation(
            self.plans,
            self.reserved_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            self.actions,
            self.reserved_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            let (message, guidance) = context.reservation();
            errors.at("ZRYNA-M3201", at, message, guidance);
            return None;
        }
        Some([self.reserved_plans.checked_add(1)?, self.reserved_actions.checked_add(actions)?])
    }

    pub(in crate::data_ownership_v1) fn release(held: [usize; 2], actions: usize) -> [usize; 2] {
        [
            held[0].checked_sub(1).expect("reserved cleanup plan"),
            held[1].checked_sub(actions).expect("reserved cleanup actions"),
        ]
    }

    pub(in crate::data_ownership_v1) fn validate_reverse(
        &self,
        action_count: usize,
        context: OwnedCleanupPlanContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            self.plans,
            self.reserved_plans.saturating_add(1),
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
            return false;
        }
        if resource_budget_violation(
            self.actions,
            self.reserved_actions.saturating_add(action_count),
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
            return false;
        }
        true
    }
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
    pub(in crate::data_ownership_v1) fn describe_reverse(
        plans: usize,
        pending: &'a [raw::PlaceId],
        excluded: Option<raw::PlaceId>,
    ) -> Option<Self> {
        let excluded_present = excluded.is_some_and(|place| pending.contains(&place));
        let action_count = pending.len() - usize::from(excluded_present);
        let id = raw::CleanupPlanId(u32::try_from(plans).ok()?);
        Some(Self { id, action_count, pending, excluded, prefix: None })
    }

    pub(in crate::data_ownership_v1) fn reverse(
        usage: &CleanupUsage,
        pending: &'a [raw::PlaceId],
        excluded: Option<raw::PlaceId>,
        context: OwnedCleanupPlanContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<Self> {
        // Keep budget rejection before checked ID derivation for the admitted live path.
        let recipe = Self::describe_reverse(usage.plans.min(u32::MAX as usize), pending, excluded)?;
        if !usage.validate_reverse(recipe.action_count, context, at, errors) {
            return None;
        }
        Some(recipe)
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
