use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::aggregate_resource_formulas::{
    aggregate_clone_budget_violation, projected_aggregate_clone_budget_violation,
    projected_string_clone_budget_violation,
};
use super::super::owned_lowering_resources::push_aggregate_clone_prefix_cleanup;
use super::super::span;
use super::super::type_model::{Binding, Ty};
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn clone_aggregate(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
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
        if !self.whole_root_available(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "clone the aggregate only before moving any owned projection",
            );
            return None;
        }

        let pending = self.owners.pending().len();
        let prefix_actions = pending.checked_add(1).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone prefix cleanup overflows its checked action count",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        let _total_actions = pending.checked_add(prefix_actions).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "aggregate clone cleanup accounting overflows",
                "reduce simultaneously live owned aggregates",
            );
            None
        })?;
        if aggregate_clone_budget_violation(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "structural clone exceeds a checked value, place, or cleanup resource limit",
                "reduce simultaneously live owned aggregates or clone sites",
            );
            return None;
        }

        self.emit_aggregate_clone(&binding, expected, at)
    }

    pub(super) fn emit_aggregate_clone(
        &mut self,
        binding: &Binding,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let cleanup = self.push_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        let element_cleanup = self.push_aggregate_clone_prefix_cleanup(at, result_owner)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::ClonePlace {
                place: binding.place,
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
            self.next_value as usize,
            self.places.len(),
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

    pub(super) fn clone_projected_string(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let projection = self.owned_place(operand)?;
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
        if !self.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected String clone source is moved or overlaps a moved subobject",
                "clone only an initialized available static String projection",
            );
            return None;
        }
        let pending = self.owners.pending().len();
        if projected_string_clone_budget_violation(
            self.next_value as usize,
            self.places.len(),
            self.instructions.len(),
            self.reserved_transitions,
            self.cleanup_plans.len(),
            self.cleanup_actions,
            pending,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected String clone exceeds a checked value, place, transition, or cleanup limit",
                "reduce simultaneously live owned aggregates or projected clone sites",
            );
            return None;
        }
        let cleanup = self.push_cleanup(at, None)?;
        self.emit(
            expected,
            at,
            raw::InstructionKind::StringClone { place: projection.place, cleanup },
        )
    }

    fn push_aggregate_clone_prefix_cleanup(
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
