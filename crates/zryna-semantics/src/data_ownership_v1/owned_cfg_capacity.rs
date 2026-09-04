use zryna_source::Span;

use super::super::Errors;
use super::super::owned_control_flow_resources::{
    OwnedCfgBudgetLimit, owned_cfg_budget_violation, owned_value_budget_violation,
};
use super::OwnedCfgState;

// Capacity-only view: neither a CFG draft nor authority to populate or emit a block.
pub(in crate::data_ownership_v1) struct OwnedCfgCapacity {
    pub(in crate::data_ownership_v1) values: usize,
    pub(in crate::data_ownership_v1) held_values: usize,
    pub(in crate::data_ownership_v1) blocks: usize,
    pub(in crate::data_ownership_v1) edges: usize,
    pub(in crate::data_ownership_v1) transitions: usize,
    pub(in crate::data_ownership_v1) held_transitions: usize,
}

impl OwnedCfgCapacity {
    pub(in crate::data_ownership_v1) fn values(
        &self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        let current = self.values.checked_add(self.held_values);
        if current.is_none_or(|current| owned_value_budget_violation(current, additional)) {
            OwnedCfgState::limit(OwnedCfgBudgetLimit::Values, at, errors);
            return false;
        }
        true
    }

    pub(in crate::data_ownership_v1) fn transitions(
        &self,
        additional: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        let Some(transitions) = self
            .transitions
            .checked_add(self.held_transitions)
            .and_then(|current| current.checked_add(additional))
        else {
            OwnedCfgState::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        };
        if owned_cfg_budget_violation(self.blocks, self.edges, transitions).is_some() {
            OwnedCfgState::limit(OwnedCfgBudgetLimit::Transitions, at, errors);
            return false;
        }
        true
    }
}
