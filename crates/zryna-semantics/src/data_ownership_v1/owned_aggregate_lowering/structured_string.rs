use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::diagnostics::span;
use super::super::owned_string_read::{StringBytes, concat_arguments, concat_optional_bytes};
use super::super::type_model::OwnedAggregatePlace;
use super::availability::{materialized_availability, parent_kind};
use super::preparation_plan::{Leaf, StringRead};
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, Ty};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_string(
        &mut self,
        id: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let inputs = match expression.kind {
            RawExpressionKind::Clone { value, .. } => vec![value],
            RawExpressionKind::Call { arguments, callee, .. } => {
                concat_arguments(&arguments, span(self.input.sources(), callee.span), self.errors)?
                    .to_vec()
            }
            _ => return None,
        };
        let incoming = self.preparation_facts.retained_string_reads.clone();
        let reservation = self.reserve_constructor_commit(ty, 0, at)?;
        let mut reads = Vec::with_capacity(inputs.len());
        for input in inputs {
            let read = self.structured_string_read(input, ty, graph)?;
            self.preparation_facts.retained_string_reads.insert(read.place);
            reads.push(read);
        }
        for read in &reads {
            self.validate_structured_read(*read, at)?;
        }
        reservation.release(self);
        let cleanup = self.push_cleanup(at, None)?;
        let leaf = match reads.as_slice() {
            [source] => Leaf::StringClone {
                source: OwnedAggregatePlace {
                    ty,
                    place: source.place,
                    root: source.root,
                    mutable: false,
                    is_root: source.place == source.root,
                },
                bytes: source.bytes,
                cleanup,
            },
            [left, right] => Leaf::StringConcat {
                left: left.place,
                right: right.place,
                bytes: concat_optional_bytes(
                    left.bytes.known(),
                    right.bytes.known(),
                    at,
                    self.errors,
                )?,
                cleanup,
            },
            _ => return None,
        };
        let result = self.consume_prepared_leaf(leaf, ty, at)?.value;
        self.preparation_facts.retained_string_reads = incoming;
        Some(result)
    }

    fn structured_string_read(
        &mut self,
        id: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<StringRead> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let (place, mut root, value) = if matches!(
            expression.kind,
            RawExpressionKind::Reference { .. }
                | RawExpressionKind::FieldAccess { .. }
                | RawExpressionKind::Index { .. }
        ) {
            let source = self.owned_place(id)?;
            if source.ty != ty {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "String operand has the wrong exact type",
                    "read one exact initialized String value",
                );
                return None;
            }
            (source.place, source.root, None)
        } else {
            let value = self.structured_value(id, ty, graph)?;
            let place = self.owners.owner(value)?;
            (place, place, Some(value))
        };
        while let Some(parent) =
            self.places.get(root.0 as usize).and_then(|place| parent_kind(&place.kind))
        {
            root = parent;
        }
        let read = StringRead {
            place,
            root,
            value,
            bytes: StringBytes::from_known(
                self.preparation_facts.string_bytes.get(&place).copied(),
            ),
        };
        self.validate_structured_read(read, at)?;
        Some(read)
    }

    fn validate_structured_read(&mut self, read: StringRead, at: Span) -> Option<()> {
        let available = materialized_availability(
            &self.owners,
            &self.moved_projections,
            &self.partial_roots,
            &self.places,
        );
        if !available.projection_available(read.place, read.root)
            || self.preparation_facts.active_borrows.values().any(|(place, access)| {
                *access == raw::BorrowAccess::Exclusive
                    && available.places_overlap(*place, read.place)
            })
            || StringBytes::from_known(
                self.preparation_facts.string_bytes.get(&read.place).copied(),
            ) != read.bytes
        {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "retained String operand is unavailable or changed",
                "preserve the exact initialized read source until the String operation completes",
            );
            return None;
        }
        Some(())
    }
}
