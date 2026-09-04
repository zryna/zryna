use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;

use super::super::aggregate_resource_formulas::{
    projected_subobject_move_budget_violation, projected_subobject_return_budget_violation,
};
use super::super::span;
use super::super::type_model::{ProjectedAggregateMoveContext, Ty};
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn preflight_aggregate_subobject_move_site(&mut self, at: Span) -> bool {
        if self.aggregate_subobject_moves == 0 {
            return true;
        }
        self.errors.at(
            "ZRYNA-M3016",
            at,
            "this checkpoint admits only one aggregate-subobject move per function",
            "move one supported Struct or fixed-array subobject into one exact direct local",
        );
        false
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
        if projection.is_root || projection.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection has the wrong exact contextual type",
                "use one exact supported Struct field or fixed-array element",
            );
            return None;
        }
        if !self.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "owned projection is unavailable or overlaps an already moved subobject",
                "move each owned field or fixed-array element at most once",
            );
            return None;
        }
        if expected.is_copy() {
            return self.emit(
                expected,
                at,
                raw::InstructionKind::CopyFromPlace { place: projection.place },
            );
        }
        let aggregate_subobject =
            matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray);
        if aggregate_subobject && aggregate_context.is_none() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "static aggregate-subobject move requires one exact direct local or final return",
                "initialize one exact private local or return the exact result type from the Struct field or constant fixed-array element",
            );
            return None;
        }
        if aggregate_subobject && !self.preflight_aggregate_subobject_move_site(at) {
            return None;
        }
        if !matches!(
            expected.category,
            TypeCategory::String | TypeCategory::Struct | TypeCategory::FixedArray
        ) || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection type is outside the static subobject move checkpoint",
                "move a String, supported Struct, or supported fixed-array field or constant element into one exact direct local",
            );
            return None;
        }
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
        let value = self.emit(
            expected,
            at,
            raw::InstructionKind::MoveFromPlace { place: projection.place },
        )?;
        if aggregate_subobject {
            self.aggregate_subobject_moves += 1;
        }
        self.moved_projections.insert(projection.place);
        self.partial_roots.insert(projection.root);
        Some(value)
    }
}
