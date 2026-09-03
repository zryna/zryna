use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::super::owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use super::super::owned_lowering_resources::{
    OwnedCleanupAccounting, OwnedCleanupPlanContext, OwnedCleanupReservationContext,
    OwnedStringPreparationBudget, preflight_owned_string_preparation,
};
use super::super::span;
use super::super::string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringEstimateError, OwnedStringEstimateOutcome,
    estimate_owned_string_expression,
};
use super::super::type_model::Ty;
use super::PrivateStringLowerer;

impl PrivateStringLowerer<'_, '_, '_> {
    pub(super) fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    pub(super) fn preparation_estimate(
        &mut self,
        id: u32,
        context: OwnedStringEstimateContext,
        at: Span,
    ) -> Option<OwnedStringEstimateOutcome> {
        match estimate_owned_string_expression(
            self.function,
            &self.bindings,
            &self.owners,
            self.ty,
            id,
            self.owners.pending().len(),
            context,
        ) {
            Ok(estimate) => Some(OwnedStringEstimateOutcome::Estimated(estimate)),
            Err(OwnedStringEstimateError::Unsupported) => {
                Some(OwnedStringEstimateOutcome::Unsupported)
            }
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3011",
                    span(self.input.sources(), reference),
                    "String source owner is no longer available",
                    "move each owned String value at most once",
                );
                None
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive owned String preparation overflows its checked resource estimate",
                    "reduce nested String-producing expressions",
                );
                None
            }
        }
    }

    pub(super) fn preflight_string_expression(&mut self, id: u32, at: Span) -> bool {
        let Some(outcome) = self.preparation_estimate(id, OwnedStringEstimateContext::Value, at)
        else {
            return false;
        };
        let OwnedStringEstimateOutcome::Estimated(estimate) = outcome else {
            return true;
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
    }

    fn preflight_place(&mut self, at: Span) -> bool {
        preflight_owned_place_capacity_with_reserved(
            self.places.len(),
            self.reserved_places,
            1,
            at,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn reserve_local_place(&mut self, at: Span) -> bool {
        if !self.preflight_place(at) {
            return false;
        }
        self.reserved_places += 1;
        true
    }

    pub(in crate::data_ownership_v1) fn release_local_place(&mut self) {
        self.reserved_places = self.reserved_places.checked_sub(1).expect("reserved local place");
    }

    pub(super) fn reserve_local_commit(&mut self, at: Span) -> bool {
        if !self.reserve_local_place(at) {
            return false;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            return false;
        }
        true
    }

    pub(super) fn release_local_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
    }

    pub(super) fn reserve_cleanup_capacity(&mut self, actions: usize, at: Span) -> bool {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .reserve_plan(actions, OwnedCleanupReservationContext::String, at, self.errors)
    }

    pub(super) fn release_cleanup_capacity(&mut self, actions: usize) {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .release_plan(actions);
    }

    pub(in crate::data_ownership_v1) fn push_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .push_reverse(
            &self.owners,
            at,
            excluded,
            OwnedCleanupPlanContext::String,
            self.errors,
        )
    }

    pub(super) fn push_instruction_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .push_instruction_reverse(
            &mut self.cfg,
            &self.owners,
            at,
            excluded,
            OwnedCleanupPlanContext::String,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn push_temporary(
        &mut self,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<(raw::ValueId, raw::PlaceId)> {
        if !self.preflight_place(at) {
            return None;
        }
        let value = raw::ValueId(self.next_value);
        let place = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        let instruction = raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: self.ty.ir, span: at }),
            span: at,
            kind,
        };
        if !self.cfg.preflight_emit(&instruction, self.errors) {
            return None;
        }
        self.next_value += 1;
        self.places.push(raw::Place {
            id: place,
            ty: self.ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        if !self.cfg.emit(instruction, self.errors) {
            return None;
        }
        let _ = self.owners.register(value, place);
        Some((value, place))
    }

    pub(super) fn push_copy_value(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        let value = raw::ValueId(self.next_value);
        let instruction = raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        };
        if !self.cfg.preflight_emit(&instruction, self.errors) {
            return None;
        }
        self.next_value = self.next_value.checked_add(1)?;
        self.cfg.emit(instruction, self.errors).then_some(value)
    }
}
