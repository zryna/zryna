use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::diagnostics::span;
use super::super::owned_lowering_resources::CleanupRecipe;
use super::super::type_model::OwnedAggregatePlace;
use super::availability::AvailabilityView;
use super::operand_decisions::{ProjectionOperation, ReferenceKind};
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, Operation};

pub(super) enum IndexedObservation {
    Unselected,
    Value(raw::ValueId),
}

impl super::PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn is_vec_index(&self, id: u32) -> bool {
        self.expression(id).is_some_and(|expression| {
            matches!(expression.kind, RawExpressionKind::Index { base, .. }
                if self.projection_expression_type(base).is_some_and(|ty| ty.category == TypeCategory::Vec))
        })
    }

    pub(super) fn lower_vec_replacement(&mut self, target: u32, value: u32) -> Option<()> {
        let RawExpressionKind::Index { base, .. } = self.expression(target)?.kind else {
            return None;
        };
        let vector = self.projection_expression_type(base)?;
        let element = self.layouts.type_by_id(vector.layout)?.referenced_type()?;
        let ty = self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        super::constructor_preparation::PreparedValue::prepare_indexed_replacement(
            self, target, value, ty,
        )?
        .consume();
        Some(())
    }
}

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn indexed_replacement(
        &mut self,
        target: u32,
        rhs: u32,
        ty: Ty,
    ) -> Option<raw::ValueId> {
        let expression = self.decisions.function.body.expressions.get(target as usize)?;
        let at = span(self.decisions.input.sources(), expression.span);
        let RawExpressionKind::Index { base, index, .. } = expression.kind else { return None };
        let source = self.resolve(base)?;
        self.available_vector(source, true, at)?;
        let start = self.steps.len();
        self.push(Operation::IndexedEnter { end: usize::MAX, result: usize::MAX }, ty, at, None);
        let integer = self.decisions.primitive(TypeCategory::I32)?;
        let index = self.walk(index, integer)?;
        let borrow = self.begin_indexed(source, index, true, ty, at)?;
        let value = self.walk(rhs, ty)?;
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let kind = if ty.is_copy() {
            raw::InstructionKind::BorrowWrite { borrow, value }
        } else {
            raw::InstructionKind::BorrowReplace { borrow, value }
        };
        let delta = if ty.is_copy() { None } else { Some(self.state.owners.transfer(value)?) };
        if let Some(delta) = delta {
            self.state.facts.apply(delta);
        }
        self.indexed_effect(kind, ty, at)?;
        self.steps.last_mut()?.owners.extend(delta);
        self.indexed_effect(raw::InstructionKind::EndBorrow { borrow }, ty, at)?;
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, ty, at, None);
        Some(value)
    }

    pub(super) fn check_access(&mut self, place: raw::PlaceId, read: bool, at: Span) -> Option<()> {
        let state = &self.state;
        let availability =
            AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
                state.parent(id)
            });
        if state.facts.active_borrows.values().any(|(region, access)| {
            availability.places_overlap(place, *region)
                && (!read || *access == raw::BorrowAccess::Exclusive)
        }) {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "Vec operation conflicts with an active whole-container access",
                "finish the indexed operation before accessing or consuming its container",
            );
            return None;
        }
        Some(())
    }

    pub(super) fn check_leaf_access(&mut self, leaf: &Leaf<'_>, at: Span) -> Option<()> {
        match leaf {
            Leaf::Reference(decision) => self.check_access(
                decision.binding.place,
                matches!(decision.kind, ReferenceKind::Copy),
                at,
            ),
            Leaf::Projection { source, operation } => {
                self.check_access(source.place, matches!(operation, ProjectionOperation::Copy), at)
            }
            Leaf::StringClone { source, .. } => self.check_access(source.place, true, at),
            Leaf::AggregateClone { source, .. }
            | Leaf::GenericClone { source, .. }
            | Leaf::IndexedCopy { source, .. } => self.check_access(*source, true, at),
            Leaf::StringConcat { left, right, .. } => {
                self.check_access(*left, true, at)?;
                self.check_access(*right, true, at)
            }
            _ => Some(()),
        }
    }

    fn available_vector(
        &mut self,
        source: OwnedAggregatePlace,
        write: bool,
        at: Span,
    ) -> Option<()> {
        let state = &self.state;
        let availability =
            AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
                state.parent(id)
            });
        if (write && !source.mutable)
            || !availability.projection_available(source.place, source.root)
            || state.moved.iter().any(|moved| availability.places_overlap(*moved, source.place))
        {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "indexed Vec is immutable for mutation, unavailable, or partially moved",
                "use one complete initialized Vec with exclusive mutation access",
            );
            return None;
        }
        self.check_access(source.place, !write, at)
    }

    pub(super) fn indexed_effect(
        &mut self,
        kind: raw::InstructionKind,
        ty: Ty,
        at: Span,
    ) -> Option<()> {
        self.state.counts[2] = self.state.counts[2].checked_add(1)?;
        match &kind {
            raw::InstructionKind::BeginIndexedBorrow { definition, .. }
            | raw::InstructionKind::BeginBorrow(definition) => {
                self.state.facts.next_borrow = definition.id.0.checked_add(1)?;
                self.state
                    .facts
                    .active_borrows
                    .insert(definition.id, (definition.place, definition.access));
            }
            raw::InstructionKind::EndBorrow { borrow } => {
                self.state.facts.active_borrows.remove(borrow)?;
            }
            _ => {}
        }
        self.push(Operation::IndexedEffect(kind), ty, at, None);
        Some(())
    }

    fn begin_indexed(
        &mut self,
        source: OwnedAggregatePlace,
        index: raw::ValueId,
        write: bool,
        ty: Ty,
        at: Span,
    ) -> Option<raw::BorrowId> {
        self.available_vector(source, write, at)?;
        if self.state.facts.active_borrows.len() >= ir::MAX_ACTIVE_BORROWS_PER_FUNCTION {
            self.decisions.errors.at(
                "ZRYNA-M3201",
                at,
                "indexed Vec operation exceeds the active borrow limit",
                "reduce simultaneously active indexed operations",
            );
            return None;
        }
        let borrow = raw::BorrowId(self.state.facts.next_borrow);
        let cleanup = self.reverse(ty, at)?;
        self.indexed_effect(
            raw::InstructionKind::BeginIndexedBorrow {
                definition: raw::BorrowDefinition {
                    id: borrow,
                    place: source.place,
                    access: if write {
                        raw::BorrowAccess::Exclusive
                    } else {
                        raw::BorrowAccess::Shared
                    },
                    span: at,
                },
                index,
                cleanup,
            },
            ty,
            at,
        )?;
        Some(borrow)
    }

    pub(super) fn indexed_read(
        &mut self,
        id: u32,
        expected: Option<Ty>,
    ) -> Option<IndexedObservation> {
        let expression = self.decisions.function.body.expressions.get(id as usize)?.clone();
        let at = span(self.decisions.input.sources(), expression.span);
        let (operand, clone) = match expression.kind {
            RawExpressionKind::Clone { value, .. } => (value, true),
            RawExpressionKind::Index { .. } => (id, false),
            _ => return Some(IndexedObservation::Unselected),
        };
        let indexed = self.decisions.function.body.expressions.get(operand as usize)?;
        let RawExpressionKind::Index { base, index, .. } = indexed.kind else {
            return Some(IndexedObservation::Unselected);
        };
        let source = self.resolve(base)?;
        if source.ty.category != TypeCategory::Vec {
            return Some(IndexedObservation::Unselected);
        }
        let element = self.decisions.layouts.type_by_id(source.ty.layout)?.referenced_type()?;
        let ty =
            self.decisions.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        if expected.is_some_and(|expected| expected != ty) || (!clone && !ty.is_copy()) {
            self.decisions.errors.at(
                "ZRYNA-M3013",
                at,
                "Vec observation requires its exact Copy element or an explicit owned clone",
                "read the exact Copy element type or explicitly clone the owned indexed element",
            );
            return None;
        }
        self.available_vector(source, false, at)?;
        let start = self.steps.len();
        self.push(Operation::IndexedEnter { end: usize::MAX, result: usize::MAX }, ty, at, None);
        let integer = self.decisions.primitive(TypeCategory::I32)?;
        let index = self.walk(index, integer)?;
        self.available_vector(source, false, at)?;
        let value = if ty.is_copy() {
            let cleanup = self.reverse(ty, at)?;
            self.emit_leaf(Leaf::IndexedCopy { source: source.place, index, cleanup }, ty, at)?
        } else {
            let borrow = self.begin_indexed(source, index, false, ty, at)?;
            self.push(Operation::CloneCapacity { aggregate: true }, ty, at, None);
            let cleanup = self.reverse(ty, at)?;
            let owner = raw::PlaceId(u32::try_from(self.state.counts[1]).ok()?);
            let recipe = CleanupRecipe::generic_clone_prefix(
                self.state.counts[4],
                self.state.owners.pending(),
                owner,
            )?;
            let (prefix, actions) = (recipe.id, recipe.action_count);
            self.state.counts[4] = self.state.counts[4].checked_add(1)?;
            self.state.counts[5] = self.state.counts[5].checked_add(actions)?;
            self.push(Operation::GenericClonePrefix { id: prefix, owner, actions }, ty, at, None);
            let value = self.emit_leaf(Leaf::IndexedClone { borrow, cleanup, prefix }, ty, at)?;
            self.indexed_effect(raw::InstructionKind::EndBorrow { borrow }, ty, at)?;
            value
        };
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, ty, at, None);
        Some(IndexedObservation::Value(value))
    }
}
