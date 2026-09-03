use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind};

use super::super::diagnostics::Errors;
use super::super::owned_cfg_state::{
    OwnedCfgState, release_owned_commit_transitions, reserve_owned_commit_transitions,
};
use super::super::owned_control_flow_shape::{
    preflight_owned_loop_body, preflight_owned_loop_exit,
};
use super::super::owned_lowering_resources::OwnedCleanupAccounting;
use super::super::owner_state::{OwnedStringBranchState, OwnerDelta, apply_owner_delta};
use super::super::span;
use super::super::string_vec_resource_estimates::{
    OwnedStringEstimateContext, OwnedStringEstimateOutcome,
};
use super::super::type_model::Binding;
use super::{PrivateStringLowerer, StringBranchTypes};

pub(in crate::data_ownership_v1) fn preflight_owned_string_loop_skeleton(
    cfg: &OwnedCfgState,
    known_bytes: &mut BTreeMap<raw::PlaceId, Option<u64>>,
    normalize_mutation_facts: bool,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    if !cfg.preflight_skeleton(3, 4, at, errors) {
        return false;
    }
    if normalize_mutation_facts {
        for known in known_bytes.values_mut() {
            *known = None;
        }
    }
    true
}

impl PrivateStringLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn reserve_loop_drop_actions(
        &mut self,
        actions: usize,
        at: Span,
    ) -> bool {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .reserve_string_loop_actions(actions, at, self.errors)
    }

    pub(in crate::data_ownership_v1) fn release_loop_drop_actions(&mut self, actions: usize) {
        OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .release_string_loop_actions(actions);
    }

    fn commit_loop_replacement(
        &mut self,
        binding: &Binding,
        prepared_value: raw::ValueId,
        prepared_owner: raw::PlaceId,
        drop_count: usize,
        incoming: &OwnedStringBranchState,
        at: Span,
    ) -> Option<()> {
        if !self.cfg.emit(
            raw::Instruction {
                result: None,
                span: at,
                kind: raw::InstructionKind::ReplacePlace {
                    place: binding.place,
                    value: prepared_value,
                },
            },
            self.errors,
        ) {
            return None;
        }
        let delta = self.owners.replace(prepared_value, binding.place)?;
        debug_assert_eq!(
            delta,
            OwnerDelta::Replaced { prepared: prepared_owner, target: binding.place }
        );
        apply_owner_delta(&mut self.known_bytes, delta);
        self.known_bytes.insert(binding.place, None);
        let temporary_reads = self
            .owners
            .pending()
            .iter()
            .copied()
            .filter(|owner| *owner != binding.place)
            .collect::<Vec<_>>();
        debug_assert_eq!(temporary_reads.len(), drop_count);
        for owner in temporary_reads.into_iter().rev() {
            if !self.cfg.emit(
                raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::DropPlace { place: owner },
                },
                self.errors,
            ) {
                return None;
            }
            OwnedCleanupAccounting::new(
                &mut self.cleanup_plans,
                &mut self.cleanup_actions,
                &mut self.reserved_cleanup_plans,
                &mut self.reserved_cleanup_actions,
            )
            .commit_action()?;
            let delta = self.owners.consume_owner(owner)?;
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_bytes != incoming.known_bytes {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "String loop replacement does not restore the exact header owner state",
                "retain the same outer String place across every backedge",
            );
            return None;
        }
        Some(())
    }

    fn lower_loop_assignment(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        incoming: &OwnedStringBranchState,
    ) -> Option<()> {
        let RawStatementKind::Assignment { target, value, .. } = statement.kind else {
            return None;
        };
        let target_expression = self.expression(target)?.clone();
        let RawExpressionKind::Reference { name } = target_expression.kind else {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), target_expression.span),
                "String loop replacement requires one root local target",
                "assign only to the single mutable outer String",
            );
            return None;
        };
        let Some(binding) = incoming.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                "String loop replacement target is not an incoming binding",
                "assign only to the single mutable outer String",
            );
            return None;
        };
        if binding.ty != self.ty || !binding.mutable || !incoming.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3015",
                span(self.input.sources(), name.span),
                "String loop replacement target is immutable, unavailable, or has the wrong type",
                "assign only to the single mutable available outer String",
            );
            return None;
        }
        if let Some(reference_span) = self.incoming_move_span(value, incoming) {
            self.errors.at(
                "ZRYNA-M3015",
                span(self.input.sources(), reference_span),
                "String loop replacement cannot consume an incoming owner while preparing its replacement",
                "prepare an independent String or explicitly clone the target",
            );
            return None;
        }
        let at = span(self.input.sources(), statement.span);
        let outcome = self.preparation_estimate(value, OwnedStringEstimateContext::Value, at)?;
        let OwnedStringEstimateOutcome::Estimated(estimate) = outcome else {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                "String loop replacement is outside checked recursive preparation",
                "use an admitted String literal, clone, concat, or private String call",
            );
            return None;
        };
        let growth = estimate.end_pending.checked_sub(self.owners.pending().len())?;
        let drop_count = growth.checked_sub(1)?;
        let transitions = drop_count.checked_add(1)?;
        if !reserve_owned_commit_transitions(&mut self.cfg, transitions, at, self.errors) {
            return None;
        }
        if !self.reserve_loop_drop_actions(drop_count, at) {
            release_owned_commit_transitions(&mut self.cfg, transitions);
            return None;
        }
        let Some((prepared_value, prepared_owner)) = self.value(value) else {
            self.release_loop_drop_actions(drop_count);
            release_owned_commit_transitions(&mut self.cfg, transitions);
            return None;
        };
        self.release_loop_drop_actions(drop_count);
        release_owned_commit_transitions(&mut self.cfg, transitions);
        self.commit_loop_replacement(
            &binding,
            prepared_value,
            prepared_owner,
            drop_count,
            incoming,
            at,
        )
    }
}

impl PrivateStringLowerer<'_, '_, '_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn lower_root_loop(
        &mut self,
        statement_id: u32,
        statement: &syntax::RawStatementSyntax,
        saw_if: bool,
        saw_loop: &mut bool,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let RawStatementKind::While { condition, body_block, .. } = &statement.kind else {
            return None;
        };
        if saw_if || *saw_loop {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "nested or repeated owned String loops are not supported",
                "use exactly one top-level while before the final return",
            );
            return None;
        }
        *saw_loop = true;
        let bool_ty = types
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::Bool)
            .copied()?;
        let at = span(self.input.sources(), statement.span);
        if !preflight_owned_loop_exit(
            self.function,
            statement_id,
            self.input.sources(),
            self.errors,
        ) {
            return None;
        }
        if !preflight_owned_loop_body(
            self.function,
            *body_block,
            false,
            self.input.sources(),
            self.errors,
        ) {
            return None;
        }
        let body = usize::try_from(*body_block)
            .ok()
            .and_then(|index| self.function.body.blocks.get(index))?;
        let mutation = match body.statements.as_slice() {
            [mutation_id] => usize::try_from(*mutation_id)
                .ok()
                .and_then(|index| self.function.body.statements.get(index))
                .filter(|statement| matches!(statement.kind, RawStatementKind::Assignment { .. }))
                .cloned(),
            _ => None,
        };
        if mutation.is_some() && self.owners.pending().len() != 1 {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned String mutation loop requires exactly one incoming owned root",
                "declare one mutable outer String before the loop",
            );
            return None;
        }
        if !preflight_owned_string_loop_skeleton(
            &self.cfg,
            &mut self.known_bytes,
            mutation.is_some(),
            at,
            self.errors,
        ) {
            return None;
        }
        let header_id = self.cfg.reserve_block(at, self.errors).expect("preflight");
        let body_id = self.cfg.reserve_block(at, self.errors).expect("preflight");
        let exit_id = self.cfg.reserve_block(at, self.errors).expect("preflight");
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target: header_id, arguments: Vec::new() }),
            },
            self.errors,
        ) {
            return None;
        }
        let incoming = OwnedStringBranchState {
            bindings: self.bindings.clone(),
            owners: self.owners.clone(),
            known_bytes: self.known_bytes.clone(),
        };
        self.cfg.begin_block(header_id, Vec::new(), at, self.errors)?;
        let condition = self.condition(*condition, bool_ty)?;
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Branch {
                    condition,
                    when_true: raw::Edge { target: body_id, arguments: Vec::new() },
                    when_false: raw::Edge { target: exit_id, arguments: Vec::new() },
                },
            },
            self.errors,
        ) {
            return None;
        }
        let branch_types = types;
        self.cfg.begin_block(body_id, Vec::new(), at, self.errors)?;
        if let Some(mutation) = mutation.as_ref() {
            self.lower_loop_assignment(mutation, &incoming)?;
        } else {
            self.lower_branch(Some(*body_block), &incoming, at, branch_types)?;
        }
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target: header_id, arguments: Vec::new() }),
            },
            self.errors,
        ) {
            return None;
        }
        self.bindings = incoming.bindings;
        self.owners = incoming.owners;
        self.known_bytes = incoming.known_bytes;
        self.cfg.begin_block(exit_id, Vec::new(), at, self.errors)?;

        Some(())
    }
}
