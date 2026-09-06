use zryna_ir::data_ownership_v1::raw;

use super::super::owned_call_resolution::OwnedCallResolution;
use super::expression_decisions::{ExpressionDecisions, ExpressionKind};
use super::structured_graph::StructuredGraph;
use super::{PrivateOwnedAggregateLowerer, Ty};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn structured_call(
        &mut self,
        id: u32,
        ty: Ty,
        graph: &mut StructuredGraph,
    ) -> Option<raw::ValueId> {
        let decision = ExpressionDecisions {
            input: self.input,
            file: self.file,
            function: self.function,
            module: self.module,
            declarations: self.declarations,
            graph: self.graph,
            node_types: self.node_types,
            layouts: self.layouts,
            errors: self.errors,
        }
        .classify_prepared(id, Some(ty), true)?;
        let ExpressionKind::Call { callee, arguments } = decision.kind else {
            if matches!(decision.kind, ExpressionKind::StringConcat { .. }) {
                return self.structured_string(id, ty, graph);
            }
            return self.value(id, ty);
        };
        let at = decision.at;
        let signature = OwnedCallResolution {
            input: self.input,
            module: self.module,
            catalog: self.catalog,
            errors: self.errors,
        }
        .lookup(callee, "call one exact private same-module function")?;
        if !signature.private
            || signature.result != ty
            || !signature
                .parameters
                .iter()
                .chain(std::iter::once(&ty))
                .chain(signature.borrow_parameters.iter().map(|parameter| &parameter.referent))
                .all(|ty| super::mixed_shape::supported(*ty, self.layouts))
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "structured call requires an exact private non-handle value signature",
                "pass exact owned or Copy values without carrying a borrow across a match edge",
            );
            return None;
        }
        if arguments.len() != signature.parameter_order.len() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameter_order.len()
                ),
                "pass every exact declared value argument in source order",
            );
            return None;
        }
        let reservation = self.reserve_constructor_commit(ty, 0, at)?;
        let arguments = self.structured_call_arguments(&signature, arguments, at, graph)?;
        reservation.release(self);
        // This tail is still in the function's unpublished scratch graph. Its exact cleanup
        // comes from reconciled post-argument owners, never the entry pending-owner count.
        let cleanup = self.push_cleanup(at, None)?;
        let emission = self.emit_recorded(
            ty,
            at,
            raw::InstructionKind::DirectCall { callee: signature.id, arguments, cleanup },
        )?;
        for delta in emission.owners {
            self.preparation_facts.apply(delta);
        }
        Some(emission.value)
    }
}
