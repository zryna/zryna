use zryna_ir::data_ownership_v1::{WeakUpgradeShape, raw};
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{RawExpressionKind, RawIdentifierSyntax};

use super::super::diagnostics::span;
use super::availability::materialized_availability;
use super::structured_graph::StructuredGraph;
use super::{Binding, PrivateOwnedAggregateLowerer, Ty};

#[derive(Clone, Copy)]
struct UpgradeOperand {
    place: raw::PlaceId,
    temporary: bool,
    shared: Ty,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn inferred_upgrade_type(&mut self, id: u32) -> Option<Ty> {
        if let Some(ty) = self.projection_expression_type(id) {
            return Some(ty);
        }
        let expression = self.expression(id)?.clone();
        match expression.kind {
            RawExpressionKind::Clone { value, .. } => self.inferred_upgrade_type(value),
            RawExpressionKind::Downgrade { value, .. } => {
                let shared = self.inferred_upgrade_type(value)?;
                let payload = self.layouts.type_by_id(shared.layout)?.referenced_type()?;
                self.node_types
                    .iter()
                    .flatten()
                    .find(|candidate| {
                        candidate.category == TypeCategory::Weak
                            && self
                                .layouts
                                .type_by_id(candidate.layout)
                                .and_then(|ty| ty.referenced_type())
                                == Some(payload)
                    })
                    .copied()
            }
            RawExpressionKind::Call { ref callee, .. } => {
                let super::super::function_catalog::FunctionResolution::Exact(signature) =
                    self.catalog.resolve(self.module, &callee.text)
                else {
                    return None;
                };
                Some(signature.result)
            }
            RawExpressionKind::Match { ref arms, .. } => {
                let mut result = None;
                for arm in arms {
                    let ty = self.inferred_upgrade_type(arm.value)?;
                    if result.is_some_and(|result| result != ty) {
                        return None;
                    }
                    result = Some(ty);
                }
                result
            }
            _ => None,
        }
    }

    fn upgrade_shared_type(&mut self, weak: Ty, at: Span) -> Option<Ty> {
        let shape = WeakUpgradeShape::derive(self.layouts, weak.layout);
        let shared = shape.and_then(|shape| {
            self.node_types
                .iter()
                .flatten()
                .find(|candidate| candidate.layout == shape.success_parameter_type())
                .copied()
        });
        if weak.category != TypeCategory::Weak || shared.is_none() {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "weak upgrade operand does not have one exact Weak type",
                "upgrade one available Weak<T> handle",
            );
            return None;
        }
        shared
    }

    fn upgrade_operand(
        &mut self,
        id: u32,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<UpgradeOperand> {
        let expression_at = span(self.input.sources(), self.expression(id)?.span);
        let addressable = matches!(
            self.expression(id)?.kind,
            RawExpressionKind::Reference { .. }
                | RawExpressionKind::FieldAccess { .. }
                | RawExpressionKind::Index { .. }
        );
        let weak = self.inferred_upgrade_type(id).or_else(|| {
            self.errors.at(
                "ZRYNA-M3013",
                expression_at,
                "weak upgrade operand does not produce an exact handle type",
                "produce one exact Weak<T> handle before upgrading it",
            );
            None
        })?;
        let shared = self.upgrade_shared_type(weak, expression_at)?;
        if addressable {
            let source = self.owned_place(id)?;
            if source.ty != weak
                || !materialized_availability(
                    &self.owners,
                    &self.moved_projections,
                    &self.partial_roots,
                    &self.places,
                )
                .projection_available(source.place, source.root)
            {
                self.errors.at(
                    "ZRYNA-M3014",
                    expression_at,
                    "weak upgrade handle is moved or unavailable",
                    "upgrade one complete initialized Weak handle",
                );
                return None;
            }
            return Some(UpgradeOperand { place: source.place, temporary: false, shared });
        }
        let value = self.structured_value(id, weak, graph)?;
        let place = self.owners.owner(value).or_else(|| {
            self.errors.at(
                "ZRYNA-M3014",
                expression_at,
                "weak upgrade producer has no available result owner",
                "produce one distinct owned Weak handle",
            );
            None
        })?;
        let _ = at;
        Some(UpgradeOperand { place, temporary: true, shared })
    }

    fn finish_upgrade_operand(&mut self, operand: UpgradeOperand, at: Span) -> Option<()> {
        if !operand.temporary {
            return Some(());
        }
        if !self.preflight_transition(1, at) {
            return None;
        }
        self.emit_prepared_effect(at, raw::InstructionKind::DropPlace { place: operand.place });
        let delta = self.owners.consume_owner(operand.place)?;
        self.preparation_facts.apply(delta);
        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn structured_weak_upgrade(
        &mut self,
        weak: u32,
        binding: &RawIdentifierSyntax,
        success_block: u32,
        expired_block: u32,
        result: Ty,
        at: Span,
        graph: &mut StructuredGraph,
    ) -> Option<bool> {
        if self
            .bindings
            .keys()
            .chain(self.preparation_facts.aliases.keys())
            .any(|name| name.eq_ignore_ascii_case(&binding.text))
        {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), binding.span),
                "weak-upgrade binding collides with a preceding binding",
                "choose one portable distinct success binding",
            );
            return None;
        }
        self.join_state(at)?;
        let operand = self.upgrade_operand(weak, at, graph)?;
        let incoming = self.join_state(at)?;
        let cleanup = self.push_cleanup(at, None)?;
        let origin = graph.terminate(
            self,
            at,
            raw::Terminator::WeakUpgradeBranch {
                weak: operand.place,
                success: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
                expired: raw::Edge { target: raw::BlockId(0), arguments: Vec::new() },
                cleanup,
            },
        )?;

        let success = graph.next(self, at)?;
        let binding_at = span(self.input.sources(), binding.span);
        let value = graph.result_parameter(self, operand.shared, binding_at)?;
        let owner = self.owners.owner(value)?;
        self.finish_upgrade_operand(operand, at)?;
        let success_falls = self.structured_scope_with_owned_binding(
            success_block,
            result,
            binding.text.clone(),
            Binding { ty: operand.shared, place: owner, mutable: false },
            binding_at,
            graph,
        )?;
        let success_state = success_falls.then(|| self.join_state(at)).flatten();
        let success_jump = success_falls.then(|| graph.jump(self, at, raw::BlockId(0))).flatten();

        self.restore_join(&incoming);
        let expired = graph.next(self, at)?;
        self.finish_upgrade_operand(operand, at)?;
        let expired_falls = self.structured_scope(expired_block, result, graph)?;
        if expired_falls && let Some(state) = &success_state {
            self.reconcile_join(state, at)?;
        }
        let expired_jump = expired_falls.then(|| graph.jump(self, at, raw::BlockId(0))).flatten();
        if !expired_falls && let Some(state) = &success_state {
            self.restore_join(state);
        }

        let raw::Terminator::WeakUpgradeBranch { success: yes, expired: no, .. } =
            &mut graph.blocks[origin].terminator.as_mut()?.kind
        else {
            return None;
        };
        yes.target = success;
        no.target = expired;
        if success_falls || expired_falls {
            let join = graph.next(self, at)?;
            if let Some(block) = success_jump {
                graph.retarget_jump(block, join);
            }
            if let Some(block) = expired_jump {
                graph.retarget_jump(block, join);
            }
            Some(true)
        } else {
            Some(false)
        }
    }
}
