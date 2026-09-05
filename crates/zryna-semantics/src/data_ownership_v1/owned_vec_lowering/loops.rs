use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawStatementKind};

use super::super::owned_control_flow_shape::{
    preflight_owned_loop_body, preflight_owned_loop_exit,
};
use super::super::owner_state::OwnedVecBranchState;
use super::PrivateVecLowerer;
use crate::data_ownership_v1::diagnostics::span;

impl PrivateVecLowerer<'_, '_, '_> {
    fn lower_loop_push(
        &mut self,
        expression_id: u32,
        incoming: &OwnedVecBranchState,
        at: Span,
    ) -> Option<()> {
        self.lower_push_effect_with_policy(expression_id, Some(incoming), true)?;
        self.bindings = incoming.bindings.clone();
        if self.owners != incoming.owners || self.known_string_bytes != incoming.known_string_bytes
        {
            self.errors.at(
                "ZRYNA-M3015",
                at,
                "Vec push loop does not restore the exact header owner state",
                "retain the same outer Vec place and consume only the pushed element",
            );
            return None;
        }
        Some(())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn lower_root_while(
        &mut self,
        statement_id: u32,
        statement: &syntax::RawStatementSyntax,
        saw_if: bool,
        saw_loop: &mut bool,
    ) -> Option<()> {
        let RawStatementKind::While { condition, body_block, .. } = &statement.kind else {
            return None;
        };
        if saw_if || *saw_loop {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "nested or repeated owned Vec loops are not supported",
                "use exactly one top-level while before the final return",
            );
            return None;
        }
        *saw_loop = true;
        let bool_ty = self
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
            true,
            self.input.sources(),
            self.errors,
        ) {
            return None;
        }
        let body = usize::try_from(*body_block)
            .ok()
            .and_then(|index| self.function.body.blocks.get(index))?;
        let push = match body.statements.as_slice() {
            [effect_id] => usize::try_from(*effect_id)
                .ok()
                .and_then(|index| self.function.body.statements.get(index))
                .and_then(|statement| match statement.kind {
                    RawStatementKind::ExpressionStatement { expression, .. } => Some(expression),
                    _ => None,
                }),
            _ => None,
        };
        if push.is_some() && self.owners.pending().len() != 1 {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "Vec mutation loop requires exactly one incoming owned root",
                "declare one mutable outer exact Vec before the loop",
            );
            return None;
        }
        if !self.cfg.preflight_skeleton(3, 4, at, self.errors) {
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
        let incoming = OwnedVecBranchState {
            bindings: self.bindings.clone(),
            owners: self.owners.clone(),
            known_string_bytes: self.known_string_bytes.clone(),
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
        self.cfg.begin_block(body_id, Vec::new(), at, self.errors)?;
        if let Some(push) = push {
            self.lower_loop_push(push, &incoming, at)?;
        } else {
            self.lower_branch(Some(*body_block), &incoming, at)?;
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
        self.known_string_bytes = incoming.known_string_bytes;
        self.cfg.begin_block(exit_id, Vec::new(), at, self.errors)?;
        Some(())
    }
}
