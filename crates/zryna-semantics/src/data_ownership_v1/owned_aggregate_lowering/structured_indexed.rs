use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_syntax::v4::RawExpressionKind;

use super::super::diagnostics::span;
use super::availability::materialized_availability;
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
        let at = span(self.input.sources(), self.expression(id)?.span);
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
        let mut suspended = None;
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
            if graph.contains_match(expression.span.start, expression.span.end) {
                if first_checked != Some(ordinal) || suspended.is_some() {
                    self.errors.at("ZRYNA-M3017", at, "indexed Match would carry transient authority across a control-flow edge", "complete Match operands before the first bounds check; do not end and reborrow indexed authority");
                    return None;
                }
                suspended = Some((ordinal, index));
            }
            let element = record.referenced_type()?;
            container =
                self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        }
        let Some((ordinal, index)) = suspended else {
            self.errors.at(
                "ZRYNA-M3017",
                at,
                "indexed Match base is not yet a prepared exact container",
                "complete the container Match in an explicit local before indexed access",
            );
            return None;
        };
        let source = self.owned_place(indices[ordinal].0)?;
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
                    *access == raw::BorrowAccess::Exclusive
                        && available.places_overlap(*place, source.place)
                })
        };
        if !ready {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "indexed container is unavailable before its Match operand",
                "retain one complete initialized container while evaluating its index",
            );
            return None;
        }
        let integer = self
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied()?;
        let incoming = self.preparation_facts.retained_indexed_reads.clone();
        self.preparation_facts.retained_indexed_reads.insert(source.place);
        let value = self.structured_value(index, integer, graph)?;
        self.preparation_facts.structured_values.insert(index, (value, integer));
        let result = self.value(id, ty)?;
        assert!(
            !self.preparation_facts.structured_values.contains_key(&index),
            "indexed continuation consumes its exact Copy handoff"
        );
        self.preparation_facts.retained_indexed_reads = incoming;
        Some(result)
    }
}
