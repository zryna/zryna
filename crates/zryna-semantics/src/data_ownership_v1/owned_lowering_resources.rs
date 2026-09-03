use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

use super::diagnostics::Errors;
use super::global_resource_limits::resource_budget_violation;
use super::owned_cfg_state::OwnedCfgState;
use super::owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use super::owner_state::OwnerState;
use super::string_vec_resource_estimates::OwnedStringPreparationEstimate;

#[derive(Clone, Copy)]
pub(super) struct OwnedStringPreparationBudget {
    pub(super) cleanup_plans: usize,
    pub(super) reserved_cleanup_plans: usize,
    pub(super) cleanup_actions: usize,
    pub(super) reserved_cleanup_actions: usize,
    pub(super) places: usize,
    pub(super) reserved_places: usize,
}

pub(super) fn preflight_owned_string_preparation(
    estimate: OwnedStringPreparationEstimate,
    budget: OwnedStringPreparationBudget,
    cfg: &mut OwnedCfgState,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    let plans = budget.cleanup_plans.checked_add(budget.reserved_cleanup_plans);
    let actions = budget.cleanup_actions.checked_add(budget.reserved_cleanup_actions);
    if plans.is_none_or(|current| {
        resource_budget_violation(
            current,
            estimate.cleanup_plans,
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        )
    }) || actions.is_none_or(|current| {
        resource_budget_violation(
            current,
            estimate.cleanup_actions,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        )
    }) {
        errors.at(
            "ZRYNA-M3201",
            at,
            "recursive owned String preparation exceeds the per-function cleanup limits",
            "reduce nested String-producing expressions or simultaneously live owners",
        );
        return false;
    }
    if cfg.reserve_values(estimate.values, at, errors).is_none() {
        return false;
    }
    cfg.release_values(estimate.values);
    if !preflight_owned_place_capacity_with_reserved(
        budget.places,
        budget.reserved_places,
        estimate.places,
        at,
        errors,
    ) {
        return false;
    }
    cfg.preflight_transitions(estimate.transitions, at, errors)
}

#[derive(Clone, Copy)]
pub(super) enum OwnedCleanupPlanContext {
    String,
    Vec,
}

impl OwnedCleanupPlanContext {
    fn reservation(self) -> (&'static str, &'static str) {
        match self {
            Self::String => (
                "reserved String cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live Strings or fallible String operations",
            ),
            Self::Vec => (
                "reserved Vec cleanup exceeds the per-function M3 limits",
                "reduce simultaneously live owned values or fallible Vec operations",
            ),
        }
    }

    fn plan_guidance(self) -> &'static str {
        match self {
            Self::String => "reduce fallible private String operations",
            Self::Vec => "reduce fallible private Vec operations",
        }
    }

    fn action_guidance(self) -> &'static str {
        match self {
            Self::String => {
                "reduce simultaneously live Strings or fallible private String operations"
            }
            Self::Vec => {
                "reduce simultaneously live owned values or fallible private Vec operations"
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum OwnedCleanupActionContext {
    StringBranchLocal,
    StringTerminalArm,
    VecBranchLocal,
    VecTerminalArm,
}

impl OwnedCleanupActionContext {
    fn diagnostic(self) -> (String, &'static str) {
        match self {
            Self::StringBranchLocal => (
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned Strings or fallible String operations",
            ),
            Self::StringTerminalArm => (
                "terminal String arm cleanup exceeds the per-function M3 limit".to_owned(),
                "reduce owned temporaries in the returning branch expression",
            ),
            Self::VecBranchLocal => (
                format!(
                    "derived cleanup actions exceed the per-function M3 limit of {}",
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION
                ),
                "reduce branch-local owned values or fallible Vec operations",
            ),
            Self::VecTerminalArm => (
                "terminal Vec arm cleanup exceeds the per-function M3 limit".to_owned(),
                "reduce owned temporaries in the returning branch expression",
            ),
        }
    }
}

pub(super) struct OwnedCleanupAccounting<'a> {
    plans: &'a mut Vec<raw::CleanupPlan>,
    committed_actions: &'a mut usize,
    reserved_plans: &'a mut usize,
    reserved_actions: &'a mut usize,
}

impl<'a> OwnedCleanupAccounting<'a> {
    pub(super) fn new(
        plans: &'a mut Vec<raw::CleanupPlan>,
        committed_actions: &'a mut usize,
        reserved_plans: &'a mut usize,
        reserved_actions: &'a mut usize,
    ) -> Self {
        Self { plans, committed_actions, reserved_plans, reserved_actions }
    }

    pub(super) fn reserve_plan(
        &mut self,
        actions: usize,
        context: OwnedCleanupPlanContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            self.plans.len(),
            self.reserved_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            *self.committed_actions,
            self.reserved_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            let (message, guidance) = context.reservation();
            errors.at("ZRYNA-M3201", at, message, guidance);
            return false;
        }
        *self.reserved_plans += 1;
        *self.reserved_actions += actions;
        true
    }

    pub(super) fn release_plan(&mut self, actions: usize) {
        *self.reserved_plans = self.reserved_plans.checked_sub(1).expect("reserved cleanup plan");
        *self.reserved_actions =
            self.reserved_actions.checked_sub(actions).expect("reserved cleanup actions");
    }

    pub(super) fn reserve_string_loop_actions(
        &mut self,
        actions: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            *self.committed_actions,
            self.reserved_actions.saturating_add(actions),
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                "reserved String loop cleanup exceeds the per-function M3 limit",
                "reduce temporary read operands in the loop replacement",
            );
            return false;
        }
        *self.reserved_actions += actions;
        true
    }

    pub(super) fn release_string_loop_actions(&mut self, actions: usize) {
        *self.reserved_actions =
            self.reserved_actions.checked_sub(actions).expect("reserved loop drop actions");
    }

    pub(super) fn preflight_actions(
        &self,
        additional: usize,
        context: OwnedCleanupActionContext,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        if resource_budget_violation(
            *self.committed_actions,
            additional,
            ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ) {
            let (message, guidance) = context.diagnostic();
            errors.at("ZRYNA-M3201", at, message, guidance);
            return false;
        }
        true
    }

    pub(super) fn commit_action(&mut self) -> Option<()> {
        self.commit_actions(1)
    }

    fn commit_actions(&mut self, additional: usize) -> Option<()> {
        *self.committed_actions = self.committed_actions.checked_add(additional)?;
        Some(())
    }

    pub(super) fn push_reverse(
        &mut self,
        owners: &OwnerState,
        at: Span,
        excluded: Option<raw::PlaceId>,
        context: OwnedCleanupPlanContext,
        errors: &mut Errors<'_>,
    ) -> Option<raw::CleanupPlanId> {
        if resource_budget_violation(
            self.plans.len(),
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
            return None;
        }
        let pending = owners.pending();
        let excluded_present = excluded.is_some_and(|place| owners.contains(place));
        let action_count = pending.len() - usize::from(excluded_present);
        if resource_budget_violation(
            *self.committed_actions,
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
            return None;
        }
        let id = raw::CleanupPlanId(u32::try_from(self.plans.len()).unwrap_or(u32::MAX));
        let actions = owners
            .pending()
            .iter()
            .rev()
            .copied()
            .filter(|place| Some(*place) != excluded)
            .map(raw::DropAction::DropPlace)
            .collect();
        self.plans.push(raw::CleanupPlan { id, span: at, actions });
        self.commit_actions(action_count).expect("preflighted cleanup action count");
        Some(id)
    }

    pub(super) fn push_instruction_reverse(
        &mut self,
        cfg: &mut OwnedCfgState,
        owners: &OwnerState,
        at: Span,
        excluded: Option<raw::PlaceId>,
        context: OwnedCleanupPlanContext,
        errors: &mut Errors<'_>,
    ) -> Option<raw::CleanupPlanId> {
        if !cfg.preflight_transition(at, errors) {
            return None;
        }
        self.push_reverse(owners, at, excluded, context, errors)
    }

    pub(super) fn push_vec_clone_prefix(
        &mut self,
        owners: &OwnerState,
        result_owner: raw::PlaceId,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> Option<raw::CleanupPlanId> {
        let action_count =
            checked_vec_clone_prefix_action_count(owners.pending().len(), at, errors)?;
        if resource_budget_violation(
            self.plans.len(),
            self.reserved_plans.saturating_add(1),
            ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        ) || resource_budget_violation(
            *self.committed_actions,
            self.reserved_actions.saturating_add(action_count),
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
        let id = raw::CleanupPlanId(u32::try_from(self.plans.len()).ok()?);
        let actions = std::iter::once(raw::DropAction::DropVecInitializedPrefix(result_owner))
            .chain(owners.pending().iter().rev().copied().map(raw::DropAction::DropPlace))
            .collect();
        self.plans.push(raw::CleanupPlan { id, span: at, actions });
        self.commit_actions(action_count).expect("preflighted cleanup action count");
        Some(id)
    }
}

pub(super) fn checked_vec_clone_prefix_action_count(
    pending_count: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<usize> {
    pending_count.checked_add(1).or_else(|| {
        errors.at(
            "ZRYNA-M3201",
            at,
            "Vec clone prefix cleanup overflows its checked action count",
            "reduce simultaneously live owned values",
        );
        None
    })
}
