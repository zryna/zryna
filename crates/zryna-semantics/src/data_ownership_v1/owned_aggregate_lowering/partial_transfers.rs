use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::aggregate_resource_formulas::{
    PartialTransferBudgetViolation, partial_assignment_budget_preflight,
    partial_return_budget_preflight, partial_transfer_budget_preflight,
};
use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn partial_local_transfer_source(
        &self,
        initializer: u32,
        expected: Ty,
    ) -> Option<raw::PlaceId> {
        if !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray) {
            return None;
        }
        let RawExpressionKind::Reference { name } = &self.expression(initializer)?.kind else {
            return None;
        };
        let binding = self.bindings.get(&name.text)?;
        (binding.ty == expected
            && self.owners.contains(binding.place)
            && self.partial_roots.contains(&binding.place))
        .then_some(binding.place)
    }

    pub(super) fn partial_return_transfer_source(
        &self,
        value: u32,
        expected: Ty,
    ) -> Option<raw::PlaceId> {
        if !matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray) {
            return None;
        }
        let RawExpressionKind::Reference { name } = &self.expression(value)?.kind else {
            return None;
        };
        let binding = self.bindings.get(&name.text)?;
        (binding.ty == expected
            && self.owners.contains(binding.place)
            && self.partial_roots.contains(&binding.place))
        .then_some(binding.place)
    }

    pub(super) fn partial_assignment_transfer_source(
        &self,
        value: u32,
        expected: Ty,
        target: raw::PlaceId,
    ) -> Option<raw::PlaceId> {
        let source = self.partial_return_transfer_source(value, expected)?;
        (source != target).then_some(source)
    }

    fn report_partial_transfer_budget(
        &mut self,
        violation: PartialTransferBudgetViolation,
        at: Span,
    ) {
        let (message, guidance) = match violation {
            PartialTransferBudgetViolation::PlaceAccounting => (
                "partial aggregate transfer place accounting overflowed".to_owned(),
                "reduce projected aggregate depth and local transfers",
            ),
            PartialTransferBudgetViolation::Values => (
                "partial aggregate transfer exceeds the per-function value limit".to_owned(),
                "reduce private aggregate expressions and transfers",
            ),
            PartialTransferBudgetViolation::Places => (
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned parameters, expressions, and local declarations",
            ),
            PartialTransferBudgetViolation::Transitions => (
                format!(
                    "derived ownership transitions exceed the per-function M3 limit of {}",
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                ),
                "reduce private aggregate expressions and assignments",
            ),
        };
        self.errors.at("ZRYNA-M3201", at, message, guidance);
    }

    pub(super) fn lower_partial_local_transfer(
        &mut self,
        source: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<raw::PlaceId> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate transfer topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and local transfers",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source, &shape);
        let existing_count = existing.iter().filter(|place| place.is_some()).count();
        let _additional_places = match partial_transfer_budget_preflight(
            self.budget_values(),
            self.budget_places(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            existing_count,
        ) {
            Ok(additional_places) => additional_places,
            Err(violation) => {
                self.report_partial_transfer_budget(violation, at);
                return None;
            }
        };
        let next_local = self.next_local.checked_add(1).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate transfer local identity overflowed",
                "reduce private aggregate local declarations",
            );
            None
        })?;

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial transfer place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial transfer temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial transfer source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);

        let local = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial transfer place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: local,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = next_local;
        let local_places = self.materialize_projection_shape(local, &shape, at);
        self.instructions.push(raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::InitializePlace { place: local, value },
        });
        let delta =
            self.owners.rename(value, local).expect("partial transfer temporary owner available");
        self.preparation_facts.apply(delta);
        self.migrate_partial_mask(temporary, local, &temporary_places, &local_places);
        Some(local)
    }

    pub(super) fn lower_partial_return_transfer(
        &mut self,
        source: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate return topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and return transfers",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source, &shape);
        let existing_count = existing.iter().filter(|place| place.is_some()).count();
        if let Err(violation) = partial_return_budget_preflight(
            self.budget_values(),
            self.budget_places(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            existing_count,
        ) {
            self.report_partial_transfer_budget(violation, at);
            return None;
        }

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len()).expect("partial return place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial return temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial return source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);
        Some(value)
    }

    pub(super) fn lower_partial_assignment_transfer(
        &mut self,
        source: raw::PlaceId,
        target: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<()> {
        let Some(shape) = self.complete_projection_shape(ty) else {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "partial aggregate assignment topology exceeds a deterministic resource limit",
                "reduce nested Struct fields, fixed-array lengths, and assignment transfers",
            );
            return None;
        };
        let source_existing = self.existing_projection_shape(source, &shape);
        let target_existing = self.existing_projection_shape(target, &shape);
        if let Err(violation) = partial_assignment_budget_preflight(
            self.budget_values(),
            self.budget_places(),
            self.instructions.len(),
            self.reserved_transitions,
            shape.len(),
            source_existing.iter().filter(|place| place.is_some()).count(),
            target_existing.iter().filter(|place| place.is_some()).count(),
        ) {
            self.report_partial_transfer_budget(violation, at);
            return None;
        }

        let source_places = self.materialize_projection_shape(source, &shape, at);
        let target_places = self.materialize_projection_shape(target, &shape, at);
        let value = raw::ValueId(self.next_value);
        self.next_value = self.next_value.checked_add(1).expect("value capacity preflighted");
        self.instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
            span: at,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        });
        let temporary = raw::PlaceId(
            u32::try_from(self.places.len())
                .expect("partial assignment place capacity preflighted"),
        );
        self.places.push(raw::Place {
            id: temporary,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Temporary(value),
        });
        self.owners.register(value, temporary).expect("fresh partial assignment temporary owner");
        let temporary_places = self.materialize_projection_shape(temporary, &shape, at);
        self.owners
            .rehome_move_result(value, source)
            .expect("partial assignment source owner available");
        self.migrate_partial_mask(source, temporary, &source_places, &temporary_places);

        self.instructions.push(raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::ReplacePlace { place: target, value },
        });
        let delta = self
            .owners
            .replace(value, target)
            .expect("partial assignment temporary owner available");
        self.preparation_facts.apply(delta);
        self.migrate_partial_mask(temporary, target, &temporary_places, &target_places);
        Some(())
    }

    pub(super) fn reference_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let decision = self.operand_decisions().reference_decision(name, expected)?;
        self.emit_reference_decision(decision, expected, at)
    }

    pub(super) fn emit_reference_decision(
        &mut self,
        decision: super::operand_decisions::ReferenceDecision,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        self.emit_reference_recorded(decision, expected, at).map(|emission| emission.value)
    }

    pub(super) fn emit_reference_recorded(
        &mut self,
        decision: super::operand_decisions::ReferenceDecision,
        expected: Ty,
        at: Span,
    ) -> Option<super::state::Emission> {
        let binding = decision.binding;
        if matches!(decision.kind, super::operand_decisions::ReferenceKind::Copy) {
            return self.emit_recorded(
                expected,
                at,
                raw::InstructionKind::CopyFromPlace { place: binding.place },
            );
        }
        let mut emission = self.emit_recorded(
            expected,
            at,
            raw::InstructionKind::MoveFromPlace { place: binding.place },
        )?;
        emission.owners.push(self.owners.rehome_move_result(emission.value, binding.place)?);
        Some(emission)
    }
}
