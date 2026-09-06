use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4::RawExpressionKind;

use super::constructor_preparation::PreparedValue;
use super::{PrivateOwnedAggregateLowerer, Ty};

pub(super) enum RefinedCallBorrow {
    NotApplicable,
    Begun(raw::BorrowId),
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_refined_call_borrow(
        &mut self,
        id: u32,
        expected: Ty,
        required: raw::BorrowAccess,
    ) -> Option<RefinedCallBorrow> {
        let expression = self.expression(id)?.clone();
        let (target, actual) = match expression.kind {
            RawExpressionKind::Borrow { value, .. } => (value, raw::BorrowAccess::Shared),
            RawExpressionKind::BorrowMut { value, .. } => (value, raw::BorrowAccess::Exclusive),
            _ => return Some(RefinedCallBorrow::NotApplicable),
        };
        let at = super::super::diagnostics::span(self.input.sources(), expression.span);
        let binding_name = self.refined_payload_binding(target, at)?;
        let binding = self.bindings.get(&binding_name)?;
        if expected.is_copy()
            || !super::mixed_shape::supported(expected, self.layouts)
            || binding.ty != expected
            || actual != required
        {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "refined payload call requires exact referent and borrow access",
                "pass borrow or borrowMut of the active payload matching the callee parameter",
            );
            return None;
        }
        let integer = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
            .copied()?;
        if !self.reserve_transition(at) {
            return None;
        }
        if actual == raw::BorrowAccess::Exclusive {
            self.bindings.get_mut(&binding_name).expect("validated refined binding").mutable = true;
        }
        let Some(prepared) = PreparedValue::prepare_lexical_begin(
            self,
            target,
            expected,
            actual == raw::BorrowAccess::Exclusive,
            integer,
        ) else {
            if let Some(binding) = self.bindings.get_mut(&binding_name) {
                binding.mutable = false;
            }
            self.release_transition();
            return None;
        };
        prepared.consume();
        if let Some(binding) = self.bindings.get_mut(&binding_name) {
            binding.mutable = false;
        }
        let borrow = raw::BorrowId(
            self.preparation_facts
                .next_borrow
                .checked_sub(1)
                .expect("prepared match payload borrow identity"),
        );
        Some(RefinedCallBorrow::Begun(borrow))
    }
}
