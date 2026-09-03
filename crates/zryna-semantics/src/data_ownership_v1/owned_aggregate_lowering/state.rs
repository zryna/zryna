use std::collections::BTreeSet;

use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::{Span, UntrustedSpan};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

use super::super::global_resource_limits::{
    aggregate_operand_budget_violation, aggregate_transition_budget_violation,
};
use super::super::owned_lowering_resources::push_aggregate_reverse_cleanup;
use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn preflight_transition(&mut self, additional: usize, at: Span) -> bool {
        if aggregate_transition_budget_violation(
            self.instructions.len(),
            self.reserved_transitions,
            additional,
        ) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            );
            return false;
        }
        true
    }

    pub(super) fn reserve_transition(&mut self, at: Span) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.reserved_transitions += 1;
        true
    }

    pub(super) fn release_transition(&mut self) {
        self.reserved_transitions = self
            .reserved_transitions
            .checked_sub(1)
            .expect("reserved aggregate assignment transition");
    }

    pub(super) fn emit_effect(&mut self, at: Span, kind: raw::InstructionKind) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.instructions.push(raw::Instruction { result: None, span: at, kind });
        true
    }

    pub(in crate::data_ownership_v1) fn push_cleanup(
        &mut self,
        at: Span,
        excluded: Option<raw::PlaceId>,
    ) -> Option<raw::CleanupPlanId> {
        push_aggregate_reverse_cleanup(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &self.owners,
            at,
            excluded,
            self.errors,
        )
    }

    pub(in crate::data_ownership_v1) fn emit(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        if !self.preflight_transition(1, at) {
            return None;
        }
        if self.next_value as usize >= ir::MAX_VALUES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived values exceed the per-function M3 limit of {}",
                    ir::MAX_VALUES_PER_FUNCTION
                ),
                "reduce private aggregate expressions",
            );
            return None;
        }
        if !ty.is_copy() && self.places.len() >= ir::MAX_PLACES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned aggregate temporaries and locals",
            );
            return None;
        }
        let value = raw::ValueId(self.next_value);
        self.next_value += 1;
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        });
        if !ty.is_copy() {
            let owner = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
            self.places.push(raw::Place {
                id: owner,
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Temporary(value),
            });
            self.owners.register(value, owner)?;
        }
        Some(value)
    }

    pub(super) fn target_consumption_span(
        &self,
        id: u32,
        target: raw::PlaceId,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && self.bindings.get(&name.text).is_some_and(|binding| {
                        binding.place == target && self.owners.contains(binding.place)
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.target_consumption_span(*value, target, false)
            }
            RawExpressionKind::StructConstruction { fields, .. } => {
                fields.iter().find_map(|field| {
                    let value = match field.kind {
                        RawFieldInitializerKind::Shorthand { value, .. }
                        | RawFieldInitializerKind::Explicit { value, .. } => value,
                    };
                    self.target_consumption_span(value, target, true)
                })
            }
            RawExpressionKind::FixedArrayConstruction { elements, .. } => elements
                .iter()
                .find_map(|element| self.target_consumption_span(*element, target, true)),
            RawExpressionKind::EnumConstruction { payload: Some(payload), .. } => {
                self.target_consumption_span(*payload, target, true)
            }
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. }
                if consumes_reference
                    && self.projection_expression_type(id).is_some_and(|ty| !ty.is_copy()) =>
            {
                self.projection_root_reference_span(*base, target)
            }
            _ => None,
        }
    }

    fn projection_root_reference_span(
        &self,
        id: u32,
        target: raw::PlaceId,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| binding.place == target) =>
            {
                Some(name.span)
            }
            RawExpressionKind::FieldAccess { base, .. } | RawExpressionKind::Index { base, .. } => {
                self.projection_root_reference_span(*base, target)
            }
            _ => None,
        }
    }

    pub(super) fn reserve_operands(&mut self, additional: usize, at: Span) -> Option<()> {
        if aggregate_operand_budget_violation(self.aggregate_operands, additional) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived aggregate operands exceed the M3 limit of {}",
                    ir::MAX_AGGREGATE_OPERANDS
                ),
                "reduce Struct fields and fixed-array elements",
            );
            return None;
        }
        self.aggregate_operands += additional;
        Some(())
    }

    pub(super) fn prevalidate_constructor_operands(
        &mut self,
        values: &[raw::ValueId],
        at: Span,
    ) -> Option<Vec<raw::ValueId>> {
        let mut seen = BTreeSet::new();
        let mut consumed = Vec::new();
        for value in values {
            let Some(owner) = self.owners.owner(*value) else { continue };
            if !self.owners.contains(owner) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "aggregate constructor operand owner is unavailable before commit",
                    "construct from only currently pending exact values",
                );
                return None;
            }
            if !seen.insert(owner) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "aggregate constructor attempts to consume one owner more than once",
                    "move each non-Copy field or element exactly once",
                );
                return None;
            }
            consumed.push(*value);
        }
        Some(consumed)
    }

    pub(super) fn commit_constructor_operands(&mut self, values: &[raw::ValueId]) {
        for value in values {
            self.owners
                .transfer(*value)
                .expect("prevalidated aggregate operand remains pending until infallible commit");
        }
    }

    pub(super) fn commit_enum(
        &mut self,
        expected: Ty,
        at: Span,
        ordinal: usize,
        payload: Option<raw::ValueId>,
    ) -> Option<raw::ValueId> {
        self.reserve_operands(usize::from(payload.is_some()), at)?;
        let operands = payload.into_iter().collect::<Vec<_>>();
        let consumed = self.prevalidate_constructor_operands(&operands, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::EnumConstruct {
                variant: u32::try_from(ordinal).ok()?,
                payload: operands.first().copied(),
                cleanup: None,
            },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }
}
