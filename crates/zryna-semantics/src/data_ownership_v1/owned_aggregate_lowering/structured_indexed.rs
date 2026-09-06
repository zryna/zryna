use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;

use super::super::diagnostics::span;
use super::availability::materialized_availability;
use super::constructor_preparation::PreparedValue;
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, Ty};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_indexed(
        &mut self,
        id: u32,
        indexed: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        if !ty.is_copy() && !matches!(self.expression(id)?.kind, RawExpressionKind::Clone { .. }) {
            let at = span(self.input.sources(), self.expression(id)?.span);
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "indexed observation requires an explicit clone of its owned element",
                "read a Copy element or explicitly clone the exact owned element",
            );
            return None;
        }
        self.structured_indexed_operation(indexed, id, ty, false, graph)
    }

    fn structured_indexed_chain(
        &mut self,
        indexed: u32,
        ty: Ty,
    ) -> Option<(Vec<(u32, u32)>, usize)> {
        let at = span(self.input.sources(), self.expression(indexed)?.span);
        let mut base = indexed;
        let mut indices = Vec::new();
        while let RawExpressionKind::Index { base: parent, index, .. } = self.expression(base)?.kind
        {
            indices.push((parent, index));
            base = parent;
        }
        indices.reverse();
        let mut container = self.indexed_expression_type(base).or_else(|| {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "indexed Match base is not yet a prepared exact container",
                "complete the container Match in an explicit local before indexed access",
            );
            None
        })?;
        let mut first_checked = None;
        for (ordinal, &(_, index)) in indices.iter().enumerate() {
            let expression = self.expression(index)?;
            let record = self.layouts.type_by_id(container.layout)?;
            let checked = container.category == TypeCategory::Vec
                || super::ordinary_indexed_array_preparation::checked_index(
                    &expression.kind,
                    record.array_length()?,
                );
            if checked && first_checked.is_none() {
                first_checked = Some(ordinal);
            }
            let element = record.referenced_type()?;
            container =
                self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        }
        if container != ty {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "indexed continuation has the wrong exact element type",
                "use the exact declared indexed element type",
            );
            return None;
        }
        Some((indices, first_checked?))
    }

    pub(super) fn structured_indexed_operation(
        &mut self,
        indexed: u32,
        result: u32,
        ty: Ty,
        write: bool,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        let at = span(self.input.sources(), self.expression(indexed)?.span);
        let (indices, first_checked) = self.structured_indexed_chain(indexed, ty)?;
        let source = self.owned_place(indices[first_checked].0)?;
        let ready = {
            let available = materialized_availability(
                &self.owners,
                &self.moved_projections,
                &self.partial_roots,
                &self.places,
            );
            (available.projection_available(source.place, source.root)
                || self
                    .bindings
                    .values()
                    .any(|binding| binding.place == source.root && binding.ty.is_copy()))
                && !self.preparation_facts.active_borrows.values().any(|(place, access)| {
                    (*access == raw::BorrowAccess::Exclusive || write)
                        && available.places_overlap(*place, source.place)
                })
        };
        if !ready || (write && !source.mutable) {
            self.errors.at("ZRYNA-M3014", at,
                "indexed container is unavailable before its Match operand",
                "retain one complete initialized container with the required access while evaluating its index");
            return None;
        }
        let integer = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied()?;
        if !write && first_checked + 1 == indices.len() {
            let index = indices[first_checked].1;
            let incoming = self.preparation_facts.retained_indexed_reads.clone();
            self.preparation_facts.retained_indexed_reads.insert(source.place);
            let value = self.structured_value(index, integer, graph)?;
            self.preparation_facts.structured_values.insert(index, (value, integer));
            let value = self.value(result, ty)?;
            self.preparation_facts.retained_indexed_reads = incoming;
            return Some(value);
        }
        let credits = 1 + usize::from(write);
        for _ in 0..credits {
            if !self.reserve_transition(at) {
                return None;
            }
        }
        let incoming = self.preparation_facts.retained_indexed_reads.clone();
        self.preparation_facts.retained_indexed_reads.insert(source.place);
        let mut parent = None;
        for &(prefix, index) in &indices[first_checked..] {
            let value = self.structured_value(index, integer, graph)?;
            self.preparation_facts.structured_values.insert(index, (value, integer));
            self.preparation_facts.retained_indexed_reads.clone_from(&incoming);
            PreparedValue::prepare_indexed_step(self, prefix, index, parent, write, integer)?
                .consume();
            if let Some(parent) = parent {
                self.preparation_facts.continued_borrows.remove(&parent);
            }
            let next = raw::BorrowId(self.preparation_facts.next_borrow.checked_sub(1)?);
            self.preparation_facts.continued_borrows.insert(next);
            parent = Some(next);
        }
        self.preparation_facts.retained_indexed_reads = incoming;
        if write {
            let value = self.structured_value(result, ty, graph)?;
            self.preparation_facts.structured_values.insert(result, (value, ty));
        }
        for _ in 0..credits {
            self.release_transition();
        }
        let borrow = parent?;
        let value =
            PreparedValue::prepare_indexed_finish(self, result, ty, borrow, write)?.consume();
        assert!(self.preparation_facts.continued_borrows.remove(&borrow));
        Some(value)
    }
}
