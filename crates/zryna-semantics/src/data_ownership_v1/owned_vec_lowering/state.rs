use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::owned_control_flow_resources::preflight_owned_place_capacity_with_reserved;
use super::super::owned_lowering_resources::{
    OwnedCleanupAccounting, OwnedCleanupPlanContext, OwnedCleanupReservationContext,
};
use super::super::owner_state::apply_owner_delta;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn preflight_place(&mut self, at: Span) -> bool {
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

    pub(in crate::data_ownership_v1) fn reserve_local_commit(&mut self, at: Span) -> bool {
        if !self.reserve_local_place(at) {
            return false;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            return false;
        }
        true
    }

    pub(in crate::data_ownership_v1) fn release_local_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
    }

    pub(in crate::data_ownership_v1) fn reserve_cleanup_capacity(
        &mut self,
        actions: usize,
        at: Span,
    ) -> bool {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .reserve_plan(actions, OwnedCleanupReservationContext::Vec, at, self.errors)
    }

    pub(in crate::data_ownership_v1) fn release_cleanup_capacity(&mut self, actions: usize) {
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
            OwnedCleanupPlanContext::Vec,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn push_instruction_cleanup(
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
            OwnedCleanupPlanContext::Vec,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn emit(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<(raw::ValueId, Option<raw::PlaceId>)> {
        if !ty.is_copy() && !self.preflight_place(at) {
            return None;
        }
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
        if !self.cfg.emit(instruction, self.errors) {
            return None;
        }
        if ty.is_copy() {
            return Some((value, None));
        }
        let owner = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        self.places.push(raw::Place {
            id: owner,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        let _ = self.owners.register(value, owner);
        Some((value, Some(owner)))
    }

    pub(in crate::data_ownership_v1) fn emit_effect(
        &mut self,
        at: Span,
        kind: raw::InstructionKind,
    ) -> bool {
        self.cfg.emit(raw::Instruction { result: None, span: at, kind }, self.errors)
    }

    pub(in crate::data_ownership_v1) fn rename_owner(
        &mut self,
        value: raw::ValueId,
        target: raw::PlaceId,
    ) -> bool {
        let Some(delta) = self.owners.rename(value, target) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }

    pub(in crate::data_ownership_v1) fn replace_owner(
        &mut self,
        value: raw::ValueId,
        target: raw::PlaceId,
    ) -> bool {
        let Some(delta) = self.owners.replace(value, target) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }

    pub(in crate::data_ownership_v1) fn transfer_owner(&mut self, value: raw::ValueId) -> bool {
        let Some(delta) = self.owners.transfer(value) else { return false };
        apply_owner_delta(&mut self.known_string_bytes, delta);
        true
    }
}
