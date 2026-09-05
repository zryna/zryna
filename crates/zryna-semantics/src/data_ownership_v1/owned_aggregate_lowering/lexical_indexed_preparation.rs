use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;

use super::super::diagnostics::span;
use super::Ty;
use super::indexed_vec_preparation::IndexedObservation;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, LexicalAlias, Operation};

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn lexical_begin(
        &mut self,
        target: u32,
        ty: Ty,
        write: bool,
    ) -> Option<raw::ValueId> {
        if let IndexedObservation::Value(value) = self.chained_lexical_begin(target, ty, write)? {
            return Some(value);
        }
        let expression = self.decisions.function.body.expressions.get(target as usize)?.clone();
        let at = span(self.decisions.input.sources(), expression.span);
        let RawExpressionKind::Index { base, index, .. } = expression.kind else {
            let source = self.resolve(target)?;
            let integer = self.decisions.primitive(TypeCategory::I32)?;
            // The affine preparation carrier is not an address or referent value.
            self.visits = self.visits.checked_add(1)?;
            let value = self.emit_leaf(Leaf::I32(0), integer, at)?;
            self.lexical_static_begin(source, ty, write, at)?;
            return Some(value);
        };
        let source = self.resolve(base)?;
        let element = self.decisions.layouts.type_by_id(source.ty.layout)?.referenced_type();
        if !matches!(source.ty.category, TypeCategory::FixedArray | TypeCategory::Vec)
            || element != Some(ty.layout)
        {
            self.decisions.errors.at(
                "ZRYNA-M3017",
                at,
                "indexed borrow annotation does not match its exact element type",
                "borrow one array or Vec element with its exact referent type",
            );
            return None;
        }
        let static_index = source.ty.category == TypeCategory::FixedArray
            && self.decisions.layouts.type_by_id(source.ty.layout)?.array_length().is_some_and(
                |length| {
                    !super::ordinary_indexed_array_preparation::checked_index(
                        &self.decisions.function.body.expressions[index as usize].kind,
                        length,
                    )
                },
            );
        let access_source = if static_index { self.resolve(target)? } else { source };
        self.available_vector(access_source, write, at)?;
        let integer = self.decisions.primitive(TypeCategory::I32)?;
        let value = self.walk(index, integer)?;
        if static_index {
            self.lexical_static_begin(access_source, ty, write, at)?;
        } else {
            self.begin_indexed(source, value, write, ty, at)?;
        }
        Some(value)
    }

    fn lexical_static_begin(
        &mut self,
        source: super::super::type_model::OwnedAggregatePlace,
        ty: Ty,
        write: bool,
        at: zryna_source::Span,
    ) -> Option<()> {
        if source.ty != ty {
            self.decisions.errors.at(
                "ZRYNA-M3017",
                at,
                "borrow annotation differs from the exact static referent",
                "use the exact root or static projection type",
            );
            return None;
        }
        self.available_vector(source, write, at)?;
        if self.state.facts.active_borrows.len() + self.state.facts.parameter_borrows.len()
            >= zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION
        {
            self.decisions.errors.at(
                "ZRYNA-M3201",
                at,
                "lexical borrow exceeds active authority limit",
                "reduce simultaneous borrows",
            );
            return None;
        }
        self.indexed_effect(
            raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(self.state.facts.next_borrow),
                place: source.place,
                access: if write {
                    raw::BorrowAccess::Exclusive
                } else {
                    raw::BorrowAccess::Shared
                },
                span: at,
            }),
            ty,
            at,
        )
    }

    pub(super) fn lexical_replacement(
        &mut self,
        alias: LexicalAlias,
        rhs: u32,
    ) -> Option<raw::ValueId> {
        let expression = self.decisions.function.body.expressions.get(rhs as usize)?;
        let at = span(self.decisions.input.sources(), expression.span);
        if alias.access != raw::BorrowAccess::Exclusive
            || !self.state.facts.borrow_active(alias.borrow)
        {
            self.decisions.errors.at(
                "ZRYNA-M3017",
                at,
                "indexed replacement requires active exclusive authority",
                "assign through a live BorrowMut alias",
            );
            return None;
        }
        let start = self.steps.len();
        self.push(
            Operation::IndexedEnter { end: usize::MAX, result: usize::MAX },
            alias.ty,
            at,
            None,
        );
        let value = self.walk(rhs, alias.ty)?;
        let result = self.steps.iter().rposition(|step| step.value == Some(value))?;
        let delta =
            if alias.ty.is_copy() { None } else { Some(self.state.owners.transfer(value)?) };
        if let Some(delta) = delta {
            self.state.facts.apply(delta);
        }
        let kind = if alias.ty.is_copy() {
            raw::InstructionKind::BorrowWrite { borrow: alias.borrow, value }
        } else {
            raw::InstructionKind::BorrowReplace { borrow: alias.borrow, value }
        };
        self.indexed_effect(kind, alias.ty, at)?;
        self.steps.last_mut()?.owners.extend(delta);
        let end = self.steps.len();
        self.steps[start].operation = Operation::IndexedEnter { end, result };
        self.push(Operation::IndexedExit, alias.ty, at, None);
        Some(value)
    }

    pub(super) fn lexical_alias_read(
        &mut self,
        id: u32,
        expected: Option<Ty>,
    ) -> Option<IndexedObservation> {
        let expression = self.decisions.function.body.expressions.get(id as usize)?;
        let at = span(self.decisions.input.sources(), expression.span);
        let (operand, clone) = match expression.kind {
            RawExpressionKind::Clone { value, .. } => (value, true),
            RawExpressionKind::Reference { .. } => (id, false),
            _ => return Some(IndexedObservation::Unselected),
        };
        let operand = self.decisions.function.body.expressions.get(operand as usize)?;
        let RawExpressionKind::Reference { name } = &operand.kind else {
            return Some(IndexedObservation::Unselected);
        };
        let Some(alias) = self.state.facts.aliases.get(&name.text).copied() else {
            return Some(IndexedObservation::Unselected);
        };
        if expected.is_some_and(|ty| ty != alias.ty)
            || (!alias.ty.is_copy() && !clone)
            || !self.state.facts.borrow_active(alias.borrow)
        {
            self.decisions.errors.at(
                "ZRYNA-M3017",
                at,
                "borrowed element cannot be moved, implicitly copied, or read with another type",
                "read a Copy element or explicitly clone its exact owned referent",
            );
            return None;
        }
        let value = if alias.ty.is_copy() {
            self.emit_leaf(Leaf::BorrowRead(alias.borrow), alias.ty, at)?
        } else {
            self.clone_indexed_borrow(alias.borrow, alias.ty, at)?
        };
        Some(IndexedObservation::Value(value))
    }
}
