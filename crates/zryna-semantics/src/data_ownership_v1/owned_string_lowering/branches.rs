use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::{Span, UntrustedSpan};
use zryna_syntax::v4::{RawExpressionKind, RawStatementKind};

use super::super::owned_lowering_resources::{OwnedCleanupAccounting, OwnedCleanupActionContext};
use super::super::owner_state::{OwnedStringBranchState, apply_owner_delta};
use super::{PrivateStringLowerer, StringBranchTypes};
use crate::data_ownership_v1::diagnostics::span;

impl PrivateStringLowerer<'_, '_, '_> {
    pub(super) fn condition(
        &mut self,
        id: u32,
        bool_ty: super::super::type_model::Ty,
    ) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        debug_assert_eq!(bool_ty.category, TypeCategory::Bool);
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } => {
                self.push_copy_value(bool_ty, at, raw::InstructionKind::BoolLiteral(value))
            }
            RawExpressionKind::Reference { name } => {
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("binding '{}' is not declared in this function", name.text),
                        "reference one exact preceding bool binding",
                    );
                    return None;
                };
                if binding.ty != bool_ty {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), name.span),
                        "owned String control-flow condition must have exact bool type",
                        "use a bool literal or preceding exact bool binding",
                    );
                    return None;
                }
                self.push_copy_value(
                    bool_ty,
                    at,
                    raw::InstructionKind::CopyFromPlace { place: binding.place },
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "owned String control-flow condition must be a bool literal or reference",
                    "use one exact Copy bool condition",
                );
                None
            }
        }
    }

    pub(in crate::data_ownership_v1) fn restore_branch_scope(
        &mut self,
        incoming: &OwnedStringBranchState,
        at: Span,
    ) -> Option<()> {
        if !self.owners.pending().starts_with(incoming.owners.pending()) {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "owned String branch changed an incoming owner",
                "leave every outer String unchanged on both branch paths",
            );
            return None;
        }
        let branch_owners = self.owners.pending()[incoming.owners.pending().len()..].to_vec();
        if !OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .preflight_actions(
            branch_owners.len(),
            OwnedCleanupActionContext::StringBranchLocal,
            at,
            self.errors,
        ) {
            return None;
        }
        if !self.cfg.preflight_transitions(branch_owners.len(), at, self.errors) {
            return None;
        }
        for owner in branch_owners.into_iter().rev() {
            let drop = raw::Instruction {
                result: None,
                span: at,
                kind: raw::InstructionKind::DropPlace { place: owner },
            };
            if !self.cfg.preflight_emit(&drop, self.errors) || !self.cfg.emit(drop, self.errors) {
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
                "owned String branch does not restore the incoming ownership state",
                "drop branch locals and leave every outer String unchanged",
            );
            return None;
        }
        Some(())
    }

    pub(super) fn drop_non_carried(&mut self, carried: raw::PlaceId, at: Span) -> Option<()> {
        let dropped = self
            .owners
            .pending()
            .iter()
            .copied()
            .filter(|owner| *owner != carried)
            .collect::<Vec<_>>();
        if !OwnedCleanupAccounting::new(
            &mut self.cleanup_plans,
            &mut self.cleanup_actions,
            &mut self.reserved_cleanup_plans,
            &mut self.reserved_cleanup_actions,
        )
        .preflight_actions(
            dropped.len(),
            OwnedCleanupActionContext::StringTerminalArm,
            at,
            self.errors,
        ) {
            return None;
        }
        if !self.cfg.preflight_transitions(dropped.len(), at, self.errors) {
            return None;
        }
        for owner in dropped.into_iter().rev() {
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
        Some(())
    }

    pub(super) fn lower_branch(
        &mut self,
        block_id: Option<u32>,
        incoming: &OwnedStringBranchState,
        at: Span,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let mut scope_span = at;
        if let Some(block_id) = block_id {
            let block = usize::try_from(block_id)
                .ok()
                .and_then(|index| self.function.body.blocks.get(index))?
                .clone();
            scope_span = span(self.input.sources(), block.span);
            for statement_id in block.statements {
                let statement = usize::try_from(statement_id)
                    .ok()
                    .and_then(|index| self.function.body.statements.get(index))?
                    .clone();
                if let RawStatementKind::LocalDeclaration { initializer, .. } = statement.kind {
                    if let Some(reference_span) = self.incoming_move_span(initializer, incoming) {
                        self.errors.at(
                            "ZRYNA-M3015",
                            span(self.input.sources(), reference_span),
                            "owned String loop or branch cannot move an incoming owner",
                            "clone the incoming String or construct the local independently",
                        );
                        return None;
                    }
                    self.lower_string_local(&statement, types)?;
                } else {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), statement.span),
                        "this branch statement is outside the bounded owned String if slice",
                        "use branch-local typed String declarations only",
                    );
                    return None;
                }
            }
        }
        self.restore_branch_scope(incoming, scope_span)
    }

    pub(super) fn incoming_move_span(
        &self,
        id: u32,
        incoming: &OwnedStringBranchState,
    ) -> Option<UntrustedSpan> {
        self.incoming_move_span_in_context(id, incoming, true)
    }

    fn incoming_move_span_in_context(
        &self,
        id: u32,
        incoming: &OwnedStringBranchState,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && incoming.bindings.get(&name.text).is_some_and(|binding| {
                        incoming.owners.contains(binding.place) && binding.ty == self.ty
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.incoming_move_span_in_context(*value, incoming, false)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments.iter().find_map(|argument| {
                    self.incoming_move_span_in_context(*argument, incoming, false)
                })
            }
            RawExpressionKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.incoming_move_span_in_context(*argument, incoming, true)),
            _ => None,
        }
    }

    pub(super) fn lower_root_branch(
        &mut self,
        statement: &zryna_syntax::v4::RawStatementSyntax,
        saw_if: &mut bool,
        saw_loop: bool,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let RawStatementKind::If { condition, then_block, else_clause, .. } = &statement.kind
        else {
            return None;
        };
        if *saw_if || saw_loop {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "nested or repeated owned String if statements are not supported",
                "use exactly one top-level if before the final return",
            );
            return None;
        }
        *saw_if = true;
        let bool_ty = types
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::Bool)
            .copied()?;
        let condition = self.condition(*condition, bool_ty)?;
        let at = span(self.input.sources(), statement.span);
        let then_id = self.cfg.reserve_block(at, self.errors)?;
        let else_id = self.cfg.reserve_block(at, self.errors)?;
        let join_id = self.cfg.reserve_block(at, self.errors)?;
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Branch {
                    condition,
                    when_true: raw::Edge { target: then_id, arguments: Vec::new() },
                    when_false: raw::Edge { target: else_id, arguments: Vec::new() },
                },
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
        self.cfg.begin_block(then_id, Vec::new(), at, self.errors)?;
        self.lower_branch(Some(*then_block), &incoming, at, types)?;
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target: join_id, arguments: Vec::new() }),
            },
            self.errors,
        ) {
            return None;
        }
        self.bindings = incoming.bindings.clone();
        self.owners = incoming.owners.clone();
        self.known_bytes = incoming.known_bytes.clone();
        self.cfg.begin_block(else_id, Vec::new(), at, self.errors)?;
        self.lower_branch(else_clause.as_ref().map(|clause| clause.block), &incoming, at, types)?;
        if !self.cfg.terminate(
            raw::SpannedTerminator {
                span: at,
                kind: raw::Terminator::Jump(raw::Edge { target: join_id, arguments: Vec::new() }),
            },
            self.errors,
        ) {
            return None;
        }
        self.bindings = incoming.bindings;
        self.owners = incoming.owners;
        self.known_bytes = incoming.known_bytes;
        self.cfg.begin_block(join_id, Vec::new(), at, self.errors)?;
        Some(())
    }
}
