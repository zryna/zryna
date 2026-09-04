use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::aggregate_resource_formulas::{
    aggregate_clone_budget_violation, projected_string_clone_budget_violation,
};
use super::super::type_model::OwnedAggregatePlace;
use super::super::{Binding, Ty, span};
use super::PrivateOwnedAggregateLowerer;
use super::operand_decisions::OperandDecisions;

pub(super) struct CloneUsage {
    pub(super) values: usize,
    pub(super) places: usize,
    pub(super) transitions: usize,
    pub(super) reserved_transitions: usize,
    pub(super) cleanup_plans: usize,
    pub(super) cleanup_actions: usize,
    pub(super) pending: usize,
}

impl CloneUsage {
    pub(super) fn validate(
        &self,
        aggregate: bool,
        at: Span,
        errors: &mut super::super::Errors<'_>,
    ) -> Option<()> {
        if aggregate { self.aggregate(at, errors) } else { self.string(at, errors) }
    }
    fn aggregate(&self, at: Span, errors: &mut super::super::Errors<'_>) -> Option<()> {
        let pending = self.pending;
        let prefix_actions = pending.checked_add(1).or_else(|| {
            errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone prefix cleanup overflows its checked action count",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        let _total_actions = pending.checked_add(prefix_actions).or_else(|| {
            errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone cleanup accounting overflows",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        if aggregate_clone_budget_violation(
            self.values,
            self.places,
            self.transitions.saturating_add(self.reserved_transitions),
            self.cleanup_plans,
            self.cleanup_actions,
            pending,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                "structural clone exceeds a checked value, place, or cleanup resource limit",
                "reduce simultaneously live owned aggregates or clone sites",
            );
            return None;
        }

        Some(())
    }
    fn string(&self, at: Span, errors: &mut super::super::Errors<'_>) -> Option<()> {
        let pending = self.pending;
        if projected_string_clone_budget_violation(
            self.values,
            self.places,
            self.transitions,
            self.reserved_transitions,
            self.cleanup_plans,
            self.cleanup_actions,
            pending,
        ) {
            errors.at(
                "ZRYNA-M3201",
                at,
                "projected String clone exceeds a checked value, place, transition, or cleanup limit",
                "reduce simultaneously live owned aggregates or projected clone sites",
            );
            return None;
        }
        Some(())
    }
}

impl<P: Fn(raw::PlaceId) -> Option<raw::PlaceId>> OperandDecisions<'_, '_, '_, P> {
    pub(super) fn aggregate_clone_decision(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
        usage: &CloneUsage,
    ) -> Option<Binding> {
        let binding = self.aggregate_clone_source(operand, expected, at)?;
        usage.validate(true, at, self.errors)?;
        Some(binding)
    }
    pub(super) fn string_clone_decision(
        &mut self,
        projection: OwnedAggregatePlace,
        expected: Ty,
        at: Span,
        usage: &CloneUsage,
    ) -> Option<OwnedAggregatePlace> {
        let projection = self.string_clone_source(projection, expected, at)?;
        usage.validate(false, at, self.errors)?;
        Some(projection)
    }

    pub(super) fn aggregate_clone_source(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<Binding> {
        if expected.is_copy()
            || !matches!(
                expected.category,
                TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
            )
            || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "structural clone requires one exact supported String-bearing aggregate",
                "clone an acyclic private Struct, Enum, or fixed array containing only bool, i32, String, and supported aggregate nodes",
            );
            return None;
        }
        let operand = self.expression(operand)?.clone();
        let RawExpressionKind::Reference { name } = operand.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), operand.span),
                "structural clone requires an addressable aggregate local root",
                "clone one available aggregate local by name",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate binding '{}' is not declared in this function", name.text),
                "clone one preceding available aggregate local",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "structural clone source has the wrong exact aggregate type",
                "clone a local with the exact contextual aggregate type",
            );
            return None;
        }
        if !self.availability.whole_root_available(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "clone the aggregate only before moving any owned projection",
            );
            return None;
        }

        Some(binding)
    }

    pub(super) fn string_clone_source(
        &mut self,
        projection: OwnedAggregatePlace,
        expected: Ty,
        at: Span,
    ) -> Option<OwnedAggregatePlace> {
        if projection.is_root
            || expected.category != TypeCategory::String
            || projection.ty != expected
        {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                "projected String clone requires one exact static String leaf",
                "clone an initialized Struct field or constant fixed-array String element",
            );
            return None;
        }
        if !self.availability.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected String clone source is moved or overlaps a moved subobject",
                "clone only an initialized available static String projection",
            );
            return None;
        }

        Some(projection)
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    #[cfg(test)]
    pub(super) fn clone_usage(&self) -> CloneUsage {
        CloneUsage {
            values: self.budget_values(),
            places: self.budget_places(),
            transitions: self.instructions.len(),
            reserved_transitions: self.reserved_transitions,
            cleanup_plans: self.cleanup_plans.len(),
            cleanup_actions: self.cleanup_actions,
            pending: self.owners.pending().len(),
        }
    }
}
