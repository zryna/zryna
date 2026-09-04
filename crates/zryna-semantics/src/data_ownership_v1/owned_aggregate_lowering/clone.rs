use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;

use super::super::aggregate_resource_formulas::projected_aggregate_clone_budget_violation;
use super::super::owned_lowering_resources::push_aggregate_clone_prefix_cleanup;
use super::super::type_model::{Binding, Ty};
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn emit_aggregate_clone(
        &mut self,
        binding: &Binding,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        self.emit_prepared_aggregate_clone(binding.place, expected, at, cleanup, element_cleanup)
            .map(|emission| emission.value)
    }

    pub(super) fn emit_prepared_aggregate_clone(
        &mut self,
        place: raw::PlaceId,
        expected: Ty,
        at: Span,
        cleanup: raw::CleanupPlanId,
        element_cleanup: raw::CleanupPlanId,
    ) -> Option<super::state::Emission> {
        self.emit_recorded(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )
    }

    pub(super) fn emit_projected_aggregate_clone(
        &mut self,
        expression: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let projection = self.owned_place(expression)?;
        debug_assert_eq!(projection.ty, expected);
        debug_assert!(!projection.is_root);
        debug_assert!(self.projection_available(projection.place, projection.root));
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: projection.place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )
    }
    pub(super) fn projected_aggregate_clone_site_available(&mut self, at: Span) -> bool {
        if self.projected_aggregate_clones == 0 {
            return true;
        }
        self.errors.at(
            "ZRYNA-M3016",
            at,
            "this checkpoint admits only one projected aggregate clone per function",
            "clone one static Struct or fixed-array subobject into one exact direct local or distinct-root static projection",
        );
        false
    }

    pub(super) fn clone_projected_aggregate_local(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        if !self.projected_aggregate_clone_site_available(at) {
            return None;
        }
        if expected.is_copy()
            || !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "projected structural clone requires one exact supported non-Copy aggregate",
                "clone an acyclic static Struct or fixed-array subobject containing only bool, i32, String, and supported aggregate nodes",
            );
            return None;
        }
        let Some(preflight) = self.owned_place_preflight(operand) else {
            let _ = self.owned_place(operand);
            return None;
        };
        if preflight.place.is_root || preflight.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "projected structural clone source has the wrong exact contextual type",
                "clone one exact supported Struct field or constant fixed-array element",
            );
            return None;
        }
        if !self.preflight_projection_available(&preflight) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate clone source is unavailable or overlaps a moved subobject",
                "clone only one initialized available static aggregate projection",
            );
            return None;
        }
        let pending = self.owners.pending().len();
        if projected_aggregate_clone_budget_violation(
            self.budget_values(),
            self.budget_places(),
            self.instructions.len(),
            self.reserved_transitions,
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
            preflight.missing,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected structural clone exceeds a checked value, place, transition, or cleanup resource limit",
                "reduce static projection depth, simultaneously live owners, or projected clone sites",
            );
            return None;
        }

        let projection = self.owned_place(operand)?;
        debug_assert_eq!(projection.ty, expected);
        debug_assert!(!projection.is_root);
        debug_assert!(self.projection_available(projection.place, projection.root));
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: projection.place,
                cleanup,
                element_cleanup: Some(element_cleanup),
            },
        )?;
        self.projected_aggregate_clones += 1;
        Some(result)
    }

    #[cfg(test)]
    pub(super) fn clone_projected_string(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let projection = self.owned_place(operand)?;
        let usage = self.clone_usage();
        let projection =
            self.operand_decisions().string_clone_decision(projection, expected, at, &usage)?;
        self.emit_string_clone(projection, expected, at)
    }

    #[cfg(test)]
    pub(super) fn emit_string_clone(
        &mut self,
        projection: super::super::type_model::OwnedAggregatePlace,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let cleanup = self.push_cleanup(at, None)?;
        self.emit_prepared_string_clone(projection, expected, at, cleanup)
            .map(|emission| emission.value)
    }

    pub(super) fn emit_prepared_string_clone(
        &mut self,
        projection: super::super::type_model::OwnedAggregatePlace,
        expected: Ty,
        at: Span,
        cleanup: raw::CleanupPlanId,
    ) -> Option<super::state::Emission> {
        self.emit_recorded(
            expected,
            at,
            raw::InstructionKind::StringClone { place: projection.place, cleanup },
        )
    }

    pub(super) fn push_aggregate_clone_prefix_cleanup(
        &mut self,
        at: Span,
        result_owner: raw::PlaceId,
    ) -> Option<raw::CleanupPlanId> {
        push_aggregate_clone_prefix_cleanup(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &self.owners,
            result_owner,
            at,
        )
    }
}
