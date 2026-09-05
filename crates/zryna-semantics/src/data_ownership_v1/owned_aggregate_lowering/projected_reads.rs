use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;

use super::super::aggregate_resource_formulas::{
    projected_subobject_move_budget_violation, projected_subobject_return_budget_violation,
};
use super::super::type_model::{ProjectedAggregateMoveContext, Ty};
use super::PrivateOwnedAggregateLowerer;
use crate::data_ownership_v1::diagnostics::span;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn preflight_aggregate_subobject_move_site(&mut self, at: Span) -> bool {
        self.operand_decisions().preflight_aggregate_subobject_move_site(at)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn projected_value(
        &mut self,
        id: u32,
        expected: Ty,
        aggregate_context: Option<ProjectedAggregateMoveContext>,
    ) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let final_return_preflight = if aggregate_context
            == Some(ProjectedAggregateMoveContext::FinalReturn)
            && matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray)
        {
            let Some(preflight) = self.owned_place_preflight(id) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "final aggregate-subobject return has no canonical static source path",
                    "return one supported Struct field or constant fixed-array element from a local root",
                );
                return None;
            };
            if preflight.place.is_root || preflight.place.ty != expected {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned projection has the wrong exact contextual type",
                    "return one exact supported Struct field or fixed-array element",
                );
                return None;
            }
            if !self.preflight_projection_available(&preflight) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "owned projection is unavailable or overlaps an already moved subobject",
                    "move each owned field or fixed-array element at most once",
                );
                return None;
            }
            if !self.supported(expected) || !self.preflight_aggregate_subobject_move_site(at) {
                return None;
            }
            let Some(shape) = self.complete_projection_shape(expected) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned subobject projection has no finite static topology",
                    "return an acyclic supported Struct or fixed-array projection",
                );
                return None;
            };
            let missing_descendants = if self.places.get(preflight.place.place.0 as usize).is_some()
            {
                self.existing_projection_shape(preflight.place.place, &shape)
                    .iter()
                    .filter(|place| place.is_none())
                    .count()
            } else {
                shape.len()
            };
            if projected_subobject_return_budget_violation(
                self.budget_values(),
                self.budget_places(),
                self.instructions.len(),
                self.reserved_transitions,
                self.cleanup_plans.len(),
                self.cleanup_actions,
                self.owners.pending().len(),
                preflight.missing,
                missing_descendants,
            ) {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "static aggregate-subobject return exceeds an M3 resource limit",
                    "reduce the canonical source path, projected topology, or preceding owned expressions",
                );
                return None;
            }
            Some(shape)
        } else {
            None
        };
        let projection = self.owned_place(id)?;
        let operation = self.operand_decisions().projection_decision(
            projection,
            expected,
            aggregate_context,
            at,
        )?;
        let aggregate_subobject = match operation {
            super::operand_decisions::ProjectionOperation::Copy => {
                return self.emit_selected_projection(projection, expected, at, &operation);
            }
            super::operand_decisions::ProjectionOperation::Move { aggregate_subobject } => {
                aggregate_subobject
            }
        };
        if aggregate_subobject {
            let Some(shape) = self.complete_projection_shape(expected) else {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "owned subobject projection has no finite static topology",
                    "move an acyclic supported Struct or fixed-array projection",
                );
                return None;
            };
            let existing = self.existing_projection_shape(projection.place, &shape);
            let missing = existing.iter().filter(|place| place.is_none()).count();
            let budget_violation = match aggregate_context {
                Some(
                    ProjectedAggregateMoveContext::DirectLocal
                    | ProjectedAggregateMoveContext::ProjectedReplacement,
                ) => projected_subobject_move_budget_violation(
                    self.budget_values(),
                    self.budget_places(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    missing,
                ),
                Some(ProjectedAggregateMoveContext::FinalReturn) | None => false,
            };
            if budget_violation {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "static aggregate-subobject move exceeds an M3 resource limit",
                    "reduce projected aggregate topology or preceding owned expressions",
                );
                return None;
            }
            self.materialize_projection_shape(
                projection.place,
                final_return_preflight.as_deref().unwrap_or(&shape),
                at,
            );
        }
        self.emit_selected_projection(projection, expected, at, &operation)
    }

    pub(super) fn emit_selected_projection(
        &mut self,
        projection: super::super::type_model::OwnedAggregatePlace,
        expected: Ty,
        at: Span,
        operation: &super::operand_decisions::ProjectionOperation,
    ) -> Option<raw::ValueId> {
        self.emit_projection_recorded(projection, expected, at, operation)
            .map(|emission| emission.value)
    }

    pub(super) fn emit_projection_recorded(
        &mut self,
        projection: super::super::type_model::OwnedAggregatePlace,
        expected: Ty,
        at: Span,
        operation: &super::operand_decisions::ProjectionOperation,
    ) -> Option<super::state::Emission> {
        let aggregate_subobject = match operation {
            super::operand_decisions::ProjectionOperation::Copy => {
                return self.emit_recorded(
                    expected,
                    at,
                    raw::InstructionKind::CopyFromPlace { place: projection.place },
                );
            }
            super::operand_decisions::ProjectionOperation::Move { aggregate_subobject } => {
                aggregate_subobject
            }
        };
        let emission = self.emit_recorded(
            expected,
            at,
            raw::InstructionKind::MoveFromPlace { place: projection.place },
        )?;
        if *aggregate_subobject {
            self.aggregate_subobject_moves += 1;
        }
        self.moved_projections.insert(projection.place);
        self.partial_roots.insert(projection.root);
        Some(emission)
    }
}
