use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::diagnostics::span;
use super::indexed_vec_preparation::IndexedObservation;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, Operation};

impl super::PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn indexed_expression_type(&self, id: u32) -> Option<Ty> {
        if let Some(ty) = self.projection_expression_type(id) {
            return Some(ty);
        }
        if let RawExpressionKind::Call { callee, .. } = &self.expression(id)?.kind {
            let super::super::function_catalog::FunctionResolution::Exact(signature) =
                self.catalog.resolve(self.module, &callee.text)
            else {
                return None;
            };
            return Some(signature.result);
        }
        let RawExpressionKind::Index { base, .. } = self.expression(id)?.kind else { return None };
        let base = self.indexed_expression_type(base)?;
        if base.category != TypeCategory::FixedArray {
            return None;
        }
        let element = self.layouts.type_by_id(base.layout)?.referenced_type()?;
        self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()
    }
}

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn chained_observation(
        &mut self,
        id: u32,
        clone: bool,
        expected: Option<Ty>,
        replacement: Option<u32>,
    ) -> Option<IndexedObservation> {
        let (base, indices) = self.indexed_chain(id)?;
        let fresh = self.fresh_indexed_type(base);
        if indices.len() < 2 && fresh.is_none() {
            return Some(IndexedObservation::Unselected);
        }
        let source = if fresh.is_none() { Some(self.resolve(base)?) } else { None };
        let base_ty = fresh.or(source.map(|source| source.ty))?;
        let mut ty = base_ty;
        let mut checked = fresh.is_some();
        for index in indices.iter().rev() {
            if ty.category != TypeCategory::FixedArray {
                return Some(IndexedObservation::Unselected);
            }
            checked |= super::ordinary_indexed_array_preparation::checked_index(
                &self.decisions.function.body.expressions.get(*index as usize)?.kind,
                self.decisions.layouts.type_by_id(ty.layout)?.array_length()?,
            );
            let element = self.decisions.layouts.type_by_id(ty.layout)?.referenced_type()?;
            ty = self
                .decisions
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.layout == element)
                .copied()?;
        }
        if !checked {
            return Some(IndexedObservation::Unselected);
        }
        let at = span(
            self.decisions.input.sources(),
            self.decisions.function.body.expressions.get(id as usize)?.span,
        );
        if fresh.is_some() && replacement.is_some() {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                at,
                "fresh indexed base is not a mutable assignment place",
                "assign through a mutable initialized binding",
            );
            return None;
        }
        if expected.is_some_and(|expected| expected != ty)
            || (replacement.is_none() && !clone && !ty.is_copy())
        {
            self.decisions.errors.at(
                "ZRYNA-M3013",
                at,
                "array observation requires its exact Copy element or an explicit owned clone",
                "read the exact Copy element type or explicitly clone the owned indexed element",
            );
            return None;
        }
        let start = self.steps.len();
        self.push(Operation::IndexedEnter { end: usize::MAX, result: usize::MAX }, ty, at, None);
        let source = if let Some(source) = source {
            source
        } else {
            self.materialize_indexed_base(base, base_ty, at)?
        };
        self.available_vector(source, replacement.is_some(), at)?;
        let integer = self.decisions.primitive(TypeCategory::I32)?;
        let mut parent = None;
        for expression in indices.into_iter().rev() {
            let index = self.walk(expression, integer)?;
            parent = Some(if let Some(parent) = parent {
                let borrow = raw::BorrowId(self.state.facts.next_borrow);
                let cleanup = self.reverse(ty, at)?;
                self.indexed_effect(
                    raw::InstructionKind::ProjectIndexedBorrow { parent, borrow, index, cleanup },
                    ty,
                    at,
                )?;
                borrow
            } else {
                self.begin_access(source, index, replacement.is_some(), ty, at)?
            });
        }
        let borrow = parent?;
        let value = self.finish_chained_access(borrow, ty, at, replacement)?;
        if fresh.is_some() {
            self.drop_indexed_base(source, at)?;
        }
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, ty, at, None);
        Some(IndexedObservation::Value(value))
    }

    fn indexed_chain(&self, mut base: u32) -> Option<(u32, Vec<u32>)> {
        let mut indices = Vec::new();
        while let RawExpressionKind::Index { base: next, index, .. } =
            self.decisions.function.body.expressions.get(base as usize)?.kind
        {
            indices.push(index);
            base = next;
        }
        Some((base, indices))
    }

    fn finish_chained_access(
        &mut self,
        borrow: raw::BorrowId,
        ty: Ty,
        at: zryna_source::Span,
        replacement: Option<u32>,
    ) -> Option<raw::ValueId> {
        let value = if let Some(rhs) = replacement {
            let value = self.walk(rhs, ty)?;
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
            value
        } else if ty.is_copy() {
            self.emit_leaf(Leaf::BorrowRead(borrow), ty, at)?
        } else {
            self.clone_indexed_borrow(borrow, ty, at)?
        };
        self.indexed_effect(raw::InstructionKind::EndBorrow { borrow }, ty, at)?;
        Some(value)
    }
}
