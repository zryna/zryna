use zryna_ir::data_ownership_v1 as ir;
use zryna_source::Span;

use super::super::diagnostics::Errors;
use super::super::global_resource_limits::{
    aggregate_operand_budget_violation, aggregate_transition_budget_violation,
    resource_budget_violation,
};
use super::super::type_model::Ty;

// These rejecting preflight views never determine emitted IDs or issue a live reservation.
pub(super) struct AggregateUsage {
    pub(super) values: usize,
    pub(super) places: usize,
    pub(super) transitions: usize,
    pub(super) operands: usize,
    pub(super) held_values: usize,
    pub(super) held_places: usize,
    pub(super) held_transitions: usize,
    pub(super) held_operands: usize,
}

impl AggregateUsage {
    pub(super) fn emit(&self, result: Ty, at: Span, errors: &mut Errors<'_>) -> bool {
        if !self.transition(1, at, errors) {
            return false;
        }
        if !self.value(at, errors) {
            return false;
        }
        if !result.is_copy() && !self.places(1, at, errors) {
            return false;
        }
        true
    }

    pub(super) fn constructor(
        &self,
        result: Ty,
        arity: usize,
        at: Span,
        errors: &mut Errors<'_>,
    ) -> bool {
        self.operands(arity, at, errors)
            && self.transition(1, at, errors)
            && self.value(at, errors)
            && self.places(usize::from(!result.is_copy()), at, errors)
    }

    pub(super) fn transition(&self, additional: usize, at: Span, errors: &mut Errors<'_>) -> bool {
        if aggregate_transition_budget_violation(
            self.transitions,
            self.held_transitions,
            additional,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            );
            return false;
        }
        true
    }

    pub(super) fn operands(&self, additional: usize, at: Span, errors: &mut Errors<'_>) -> bool {
        if aggregate_operand_budget_violation(
            self.operands.saturating_add(self.held_operands),
            additional,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived aggregate operands exceed the M3 limit of {}",
                    ir::MAX_AGGREGATE_OPERANDS
                ),
                "reduce Struct fields and fixed-array elements",
            );
            return false;
        }
        true
    }

    pub(super) fn value(&self, at: Span, errors: &mut Errors<'_>) -> bool {
        if self.values.saturating_add(self.held_values) >= ir::MAX_VALUES_PER_FUNCTION {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived values exceed the per-function M3 limit of {}",
                    ir::MAX_VALUES_PER_FUNCTION
                ),
                "reduce private aggregate expressions",
            );
            return false;
        }
        true
    }

    pub(super) fn places(&self, additional: usize, at: Span, errors: &mut Errors<'_>) -> bool {
        if resource_budget_violation(
            self.places.saturating_add(self.held_places),
            additional,
            ir::MAX_PLACES_PER_FUNCTION,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned aggregate temporaries and locals",
            );
            return false;
        }
        true
    }
}
