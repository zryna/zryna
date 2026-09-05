use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::type_model::{OwnedAggregatePlace, Ty};
use super::availability::AvailabilityView;
use super::operand_decisions::ProjectionOperation;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn static_replacement(
        &mut self,
        target: u32,
        rhs: u32,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let source = self.resolve(target)?;
        self.static_replacement_target(source, ty, at)?;
        let value = self.walk(rhs, ty)?;
        self.static_replacement_target(source, ty, at)?;
        if ty.is_copy() {
            self.copy_projection_write(source, value, ty, at)?;
            return Some(value);
        }
        let delta = if ty.is_copy() { None } else { Some(self.state.owners.transfer(value)?) };
        if let Some(delta) = delta {
            self.state.facts.apply(delta);
        }
        self.state.counts[2] = self.state.counts[2].checked_add(1)?;
        self.push(Operation::ReplaceProjection { place: source.place, value }, ty, at, None);
        self.steps.last_mut()?.owners.extend(delta);
        Some(value)
    }

    fn static_replacement_target(
        &mut self,
        source: OwnedAggregatePlace,
        ty: Ty,
        at: Span,
    ) -> Option<()> {
        if source.is_root
            || source.ty != ty
            || !source.mutable
            || !self.generic_projection_available(source)
        {
            self.decisions.errors.at(
                "ZRYNA-M3014", at,
                "static replacement target is immutable, unavailable, or consumed during preparation",
                "retain the exact complete mutable subobject until replacement commits",
            );
            return None;
        }
        self.check_access(source.place, false, at)
    }

    pub(super) fn generic_projection_decision(
        &mut self,
        source: OwnedAggregatePlace,
        ty: Ty,
        at: Span,
    ) -> Option<ProjectionOperation> {
        if source.is_root || source.ty != ty {
            self.decisions.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection has the wrong exact contextual type",
                "use one exact supported Struct field or fixed-array element",
            );
            return None;
        }
        if !self.generic_projection_available(source) {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "owned projection is unavailable or overlaps an already moved subobject",
                "move each owned field or fixed-array element at most once",
            );
            return None;
        }
        // Shared preparation records the exact path and moved mask. It does not acquire
        // the legacy one-site aggregate topology expansion or its independent counter.
        Some(if ty.is_copy() {
            ProjectionOperation::Copy
        } else {
            ProjectionOperation::GenericMove
        })
    }

    fn generic_projection_available(&self, source: OwnedAggregatePlace) -> bool {
        // This producer publishes parameters and local bindings only after initialization.
        // Copy roots never enter pending ownership and cannot be consumed by a value read.
        let copy_root = self
            .bindings
            .values()
            .any(|binding| binding.place == source.root && binding.ty.is_copy());
        let state = &self.state;
        copy_root
            || AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
                state.parent(id)
            })
            .projection_available(source.place, source.root)
    }

    fn copy_projection_write(
        &mut self,
        source: OwnedAggregatePlace,
        value: raw::ValueId,
        ty: Ty,
        at: Span,
    ) -> Option<()> {
        if self.state.facts.active_borrows.len()
            >= zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION
        {
            self.decisions.errors.at(
                "ZRYNA-M3201",
                at,
                "static Copy replacement exceeds the active borrow limit",
                "finish outstanding accesses before replacing the Copy subobject",
            );
            return None;
        }
        let borrow = raw::BorrowId(self.state.facts.next_borrow);
        self.indexed_effect(
            raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: borrow,
                place: source.place,
                access: raw::BorrowAccess::Exclusive,
                span: at,
            }),
            ty,
            at,
        )?;
        self.indexed_effect(raw::InstructionKind::BorrowWrite { borrow, value }, ty, at)?;
        self.indexed_effect(raw::InstructionKind::EndBorrow { borrow }, ty, at)
    }
}
