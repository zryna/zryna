use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4::RawExpressionKind;

use super::super::aggregate_resource_formulas::aggregate_clone_budget_violation;
use super::super::diagnostics::span;
use super::super::owned_lowering_resources::push_generic_clone_prefix_cleanup;
use super::constructor_preparation::PreparedValue;
use super::{PrivateOwnedAggregateLowerer, Ty};

pub(super) enum StructuredBorrowOutcome {
    NotApplicable,
    Emitted(raw::ValueId),
}

enum RefinedBorrowShape {
    NotApplicable,
    Inline { target: u32, access: raw::BorrowAccess, at: zryna_source::Span },
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn refined_borrow_shape(&self, id: u32) -> Option<RefinedBorrowShape> {
        let expression = self.expression(id)?;
        let RawExpressionKind::Clone { value, .. } = expression.kind else {
            return Some(RefinedBorrowShape::NotApplicable);
        };
        let borrow = self.expression(value)?;
        let (target, access) = match borrow.kind {
            RawExpressionKind::Borrow { value, .. } => (value, raw::BorrowAccess::Shared),
            RawExpressionKind::BorrowMut { value, .. } => (value, raw::BorrowAccess::Exclusive),
            _ => return Some(RefinedBorrowShape::NotApplicable),
        };
        Some(RefinedBorrowShape::Inline {
            target,
            access,
            at: span(self.input.sources(), expression.span),
        })
    }

    fn refined_payload_binding(&mut self, target: u32, at: zryna_source::Span) -> Option<String> {
        let RawExpressionKind::Reference { name } = &self.expression(target)?.kind else {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "inline refined borrowing requires one active match payload binding",
                "borrow the exact payload name bound by this exhaustive match arm",
            );
            return None;
        };
        let name = name.text.clone();
        let Some(binding) = self.bindings.get(&name) else {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "inline refined borrowing requires one active match payload binding",
                "borrow the exact payload name bound by this exhaustive match arm",
            );
            return None;
        };
        if !matches!(
            self.places.get(binding.place.0 as usize)?.kind,
            raw::PlaceKind::EnumPayload { .. }
        ) {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "inline refined borrowing requires one active match payload binding",
                "borrow the exact payload name bound by this exhaustive match arm",
            );
            return None;
        }
        Some(name)
    }

    pub(super) fn structured_refined_borrow_clone(
        &mut self,
        id: u32,
        expected: Ty,
    ) -> Option<StructuredBorrowOutcome> {
        let RefinedBorrowShape::Inline { target, access, at } = self.refined_borrow_shape(id)?
        else {
            return Some(StructuredBorrowOutcome::NotApplicable);
        };
        if expected.is_copy() || !super::mixed_shape::supported(expected, self.layouts) {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "refined payload borrow clone requires one exact supported owned type",
                "clone the active owned payload through borrow or borrowMut",
            );
            return None;
        }
        let integer = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
            .copied()?;
        let binding_name = self.refined_payload_binding(target, at)?;
        if aggregate_clone_budget_violation(
            self.budget_values(),
            self.budget_places(),
            self.instructions.len(),
            self.cleanup_plans.len(),
            self.cleanup_actions,
            self.owners.pending().len(),
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "refined payload clone exceeds a checked value, place, transition, or cleanup resource limit",
                "reduce simultaneously live owners or refined payload clone sites",
            );
            return None;
        }
        let mutable_binding = (access == raw::BorrowAccess::Exclusive).then_some(binding_name);
        if !self.reserve_transition(at) {
            return None;
        }
        if let Some(name) = &mutable_binding {
            self.bindings.get_mut(name).expect("validated refined binding").mutable = true;
        }
        let Some(prepared) = PreparedValue::prepare_lexical_begin(
            self,
            target,
            expected,
            access == raw::BorrowAccess::Exclusive,
            integer,
        ) else {
            if let Some(name) = &mutable_binding
                && let Some(binding) = self.bindings.get_mut(name)
            {
                binding.mutable = false;
            }
            self.release_transition();
            return None;
        };
        prepared.consume();
        if let Some(name) = &mutable_binding
            && let Some(binding) = self.bindings.get_mut(name)
        {
            binding.mutable = false;
        }
        let borrow = raw::BorrowId(
            self.preparation_facts.next_borrow.checked_sub(1).expect("prepared borrow identity"),
        );
        let cleanup = self.push_cleanup(at, None).expect("preflighted clone cleanup");
        let result_owner = raw::PlaceId(
            u32::try_from(self.places.len()).expect("preflighted clone owner identity"),
        );
        let prefix = push_generic_clone_prefix_cleanup(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &self.owners,
            result_owner,
            at,
        )
        .expect("preflighted clone prefix cleanup");
        let result = self
            .emit(
                expected,
                at,
                raw::InstructionKind::GenericCloneBorrow {
                    borrow,
                    cleanup,
                    prefix_cleanup: prefix,
                },
            )
            .expect("preflighted refined clone emission");
        self.release_transition();
        self.emit_prepared_effect(at, raw::InstructionKind::EndBorrow { borrow });
        self.preparation_facts.active_borrows.remove(&borrow).expect("prepared borrow authority");
        Some(StructuredBorrowOutcome::Emitted(result))
    }
}
