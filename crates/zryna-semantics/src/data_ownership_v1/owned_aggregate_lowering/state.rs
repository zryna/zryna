use zryna_ir::data_ownership_v1::raw;
use zryna_source::{Span, UntrustedSpan};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializerKind};

use super::super::owned_constructor_plan::{
    ConstructorKind, ConstructorPlanError, ConstructorShape, PreparedConstructor,
};
use super::super::owned_lowering_resources::push_aggregate_reverse_cleanup;
use super::super::owner_state::OwnerDelta;
use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;

pub(super) struct Emission {
    pub(super) value: raw::ValueId,
    pub(super) owners: Vec<OwnerDelta>,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn preflight_transition(&mut self, additional: usize, at: Span) -> bool {
        self.resource_usage().transition(additional, at, self.errors)
    }

    pub(super) fn reserve_transition(&mut self, at: Span) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.credit_ledger().acquire_assignment();
        true
    }

    pub(super) fn release_transition(&mut self) {
        self.credit_ledger().release_assignment();
    }

    pub(super) fn emit_effect(&mut self, at: Span, kind: raw::InstructionKind) -> bool {
        if !self.preflight_transition(1, at) {
            return false;
        }
        self.instructions.push(raw::Instruction { result: None, span: at, kind });
        true
    }

    pub(super) fn push_cleanup(
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

    pub(super) fn emit(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        self.emit_recorded(ty, at, kind).map(|emission| emission.value)
    }

    pub(super) fn emit_recorded(
        &mut self,
        ty: Ty,
        at: Span,
        kind: raw::InstructionKind,
    ) -> Option<Emission> {
        if !self.resource_usage().emit(ty, at, self.errors) {
            return None;
        }
        let value = raw::ValueId(self.next_value);
        self.next_value += 1;
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind,
        });
        let mut owners = Vec::new();
        if !ty.is_copy() {
            let owner = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
            self.places.push(raw::Place {
                id: owner,
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Temporary(value),
            });
            owners.push(self.owners.register(value, owner)?);
        }
        Some(Emission { value, owners })
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
        if !self.preflight_constructor_operands(additional, at) {
            return None;
        }
        self.aggregate_operands = self
            .aggregate_operands
            .checked_add(additional)
            .expect("constructor operand capacity preflighted");
        Some(())
    }

    pub(super) fn commit_constructor(
        &mut self,
        expected: Ty,
        kind: ConstructorKind,
        values: &[raw::ValueId],
        at: Span,
    ) -> Option<Emission> {
        self.reserve_operands(values.len(), at)?;
        let prepared = ConstructorShape::derive(self.layouts, expected, kind, values.len(), |id| {
            self.node_types.iter().flatten().find(|ty| ty.layout == id).copied()
        })
        .and_then(|shape| {
            self.constructor_types.observe(&self.instructions)?;
            shape.prepare(values, |value| self.constructor_types.get(value), &self.owners)
        });
        let prepared = constructor_result(prepared, at, self.errors)?;
        let instruction = prepared.instruction(None).expect("infallible aggregate constructor");
        let mut emission = self.emit_recorded(prepared.result_type(), at, instruction)?;
        emission.owners.extend(prepared.commit(&mut self.owners));
        Some(emission)
    }
}

pub(super) fn constructor_result(
    prepared: Result<PreparedConstructor, ConstructorPlanError>,
    at: Span,
    errors: &mut super::super::Errors<'_>,
) -> Option<PreparedConstructor> {
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(ConstructorPlanError::DuplicateOwner) => {
            errors.at(
                "ZRYNA-M3014",
                at,
                "aggregate constructor attempts to consume one owner more than once",
                "move each non-Copy field or element exactly once",
            );
            return None;
        }
        Err(_) => {
            errors.at(
                "ZRYNA-M3014",
                at,
                "aggregate constructor operand owner is unavailable before commit",
                "construct from only currently pending exact values",
            );
            return None;
        }
    };
    Some(prepared)
}
