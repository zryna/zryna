use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4::{RawStatementKind, RawStatementSyntax};

use super::super::diagnostics::span;
use super::structured_graph::StructuredGraph;
use super::{Binding, PrivateOwnedAggregateLowerer, Ty};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_match_local(
        &mut self,
        statement: &RawStatementSyntax,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<bool> {
        let RawStatementKind::LocalDeclaration { name, mutable, initializer, .. } = &statement.kind
        else {
            return Some(false);
        };
        let expression = self.expression(*initializer)?;
        if !graph.contains_match(expression.span.start, expression.span.end) {
            return Some(false);
        }
        let at = span(self.input.sources(), statement.span);
        if self
            .bindings
            .keys()
            .chain(self.preparation_facts.aliases.keys())
            .any(|bound| bound.eq_ignore_ascii_case(&name.text))
        {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                "match result binding collides with a preceding binding",
                "use one portable distinct result name",
            );
            return None;
        }
        if !self.resource_usage().places(1, at, self.errors) || !self.reserve_transition(at) {
            return None;
        }
        let value = self.structured_value(*initializer, ty, graph)?;
        self.release_transition();
        if !self.resource_usage().places(1, at, self.errors) || !self.preflight_transition(1, at) {
            return None;
        }
        let place = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        self.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local += 1;
        self.emit_prepared_effect(at, raw::InstructionKind::InitializePlace { place, value });
        if !ty.is_copy() {
            let delta = self.owners.rename(value, place)?;
            self.preparation_facts.apply(delta);
        }
        self.bindings.insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
        Some(true)
    }
}
