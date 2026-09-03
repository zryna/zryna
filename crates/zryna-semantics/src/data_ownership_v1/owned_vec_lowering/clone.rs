use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::owned_lowering_resources::{
    OwnedCleanupAccounting, checked_vec_clone_prefix_action_count,
};
use super::super::span;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::data_ownership_v1) fn clone_vec(
        &mut self,
        operand: u32,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        if !matches!(
            self.element.category,
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String
        ) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "Vec clone is sealed to exact Vec<bool>, Vec<i32>, and Vec<String>",
                "use clone only with one admitted exact private Vec element type",
            );
            return None;
        }
        let operand = self.expression(operand)?.clone();
        let RawExpressionKind::Reference { name } = operand.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), operand.span),
                "Vec clone requires an addressable local root",
                "clone one available Vec local by name",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("Vec binding '{}' is not declared in this function", name.text),
                "clone one preceding available Vec local",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), name.span),
                "Vec clone source has the wrong exact container type",
                "clone a local with the exact contextual Vec element type",
            );
            return None;
        }
        if !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("Vec value '{}' was already moved", name.text),
                "clone the Vec only while its owner remains available",
            );
            return None;
        }
        let actions = self.owners.pending().len();
        let clones_non_copy_elements = self.element.category == TypeCategory::String;
        let prefix_actions = if clones_non_copy_elements {
            Some(checked_vec_clone_prefix_action_count(actions, at, self.errors)?)
        } else {
            None
        };
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if !self.reserve_cleanup_capacity(actions, at) {
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if prefix_actions
            .is_some_and(|prefix_actions| !self.reserve_cleanup_capacity(prefix_actions, at))
        {
            self.release_cleanup_capacity(actions);
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if let Some(prefix_actions) = prefix_actions {
            self.release_cleanup_capacity(prefix_actions);
        }
        self.release_cleanup_capacity(actions);
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
        let cleanup = self.push_instruction_cleanup(at, None)?;
        let result_owner = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let element_cleanup = if clones_non_copy_elements {
            Some(self.push_vec_clone_prefix_cleanup(at, result_owner)?)
        } else {
            None
        };
        Some(
            self.emit(
                expected,
                at,
                raw::InstructionKind::VecClone { place: binding.place, cleanup, element_cleanup },
            )?
            .0,
        )
    }

    fn push_vec_clone_prefix_cleanup(
        &mut self,
        at: Span,
        result_owner: raw::PlaceId,
    ) -> Option<raw::CleanupPlanId> {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .push_vec_clone_prefix(&self.owners, result_owner, at, self.errors)
    }
}
