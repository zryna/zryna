use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind};

use super::super::layout_graph::semantic_type;
use super::super::owned_cfg_state::{
    release_owned_commit_transition, reserve_owned_commit_transition,
};
use super::super::owner_state::{OwnedVecBranchState, apply_owner_delta};
use super::super::span;
use super::super::string_vec_resource_estimates::vec_push_target_invalid;
use super::super::type_model::Binding;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    fn push_scope_error(
        incoming: Option<&OwnedVecBranchState>,
        place: raw::PlaceId,
        allow_incoming_target: bool,
    ) -> Option<(&'static str, &'static str)> {
        let incoming = incoming?;
        let is_incoming_target = incoming.bindings.values().any(|outer| outer.place == place);
        if allow_incoming_target && !is_incoming_target {
            return Some((
                "owned Vec loop must mutate its one incoming Vec root",
                "push only into the mutable Vec declared before this loop",
            ));
        }
        if !allow_incoming_target && is_incoming_target {
            return Some((
                "owned Vec branch cannot mutate an incoming Vec",
                "push only into a Vec declared inside this branch",
            ));
        }
        None
    }

    pub(in crate::data_ownership_v1) fn lower_local(
        &mut self,
        statement: &syntax::RawStatementSyntax,
    ) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        if self.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text)) {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let ty = semantic_type(
            self.file,
            *type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if !matches!(ty.category, TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String)
            && ty != self.vec_ty
        {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), statement.span),
                "local type is outside this private Vec slice",
                "use the exact Vec type or its bool, i32, or String element type",
            );
            return None;
        }
        let local_span = span(self.input.sources(), statement.span);
        if !self.reserve_local_commit(local_span) {
            return None;
        }
        let Some(value) = self.value(*initializer, ty) else {
            self.release_local_commit();
            return None;
        };
        let place = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let initialize = raw::Instruction {
            result: None,
            span: local_span,
            kind: raw::InstructionKind::InitializePlace { place, value },
        };
        self.release_local_commit();
        self.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: local_span,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = self.next_local.checked_add(1)?;
        if !self.cfg.emit(initialize, self.errors) {
            return None;
        }
        if !ty.is_copy() && !self.rename_owner(value, place) {
            self.errors.at(
                "ZRYNA-M3014",
                local_span,
                "owned local initializer has no available owner",
                "initialize the local from one available owned value",
            );
            return None;
        }
        self.bindings.insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
        Some(())
    }

    pub(in crate::data_ownership_v1) fn lower_push_effect(
        &mut self,
        expression_id: u32,
        incoming: Option<&OwnedVecBranchState>,
    ) -> Option<()> {
        self.lower_push_effect_with_policy(expression_id, incoming, false)
    }

    pub(in crate::data_ownership_v1) fn lower_push_effect_with_policy(
        &mut self,
        expression_id: u32,
        incoming: Option<&OwnedVecBranchState>,
        allow_incoming_target: bool,
    ) -> Option<()> {
        let expression = self.expression(expression_id)?.clone();
        let at = span(self.input.sources(), expression.span);
        let RawExpressionKind::VecPush { vector, value, .. } = expression.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "only push(vector, value) is admitted as a Vec effect statement",
                "use push on one mutable initialized Vec local",
            );
            return None;
        };
        let vector_expression = self.expression(vector)?.clone();
        let RawExpressionKind::Reference { name } = vector_expression.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), vector_expression.span),
                "push requires an addressable Vec local",
                "push into one mutable initialized Vec local",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("Vec binding '{}' is not declared in this function", name.text),
                "reference one exact preceding mutable Vec local",
            );
            return None;
        };
        if let Some((message, help)) =
            Self::push_scope_error(incoming, binding.place, allow_incoming_target)
        {
            self.errors.at("ZRYNA-M3015", span(self.input.sources(), name.span), message, help);
            return None;
        }
        if binding.ty != self.vec_ty {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), name.span),
                "push target has the wrong exact Vec type",
                "push into the function's exact Vec type",
            );
            return None;
        }
        if vec_push_target_invalid(binding.mutable, self.owners.contains(binding.place)) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "push target is immutable, uninitialized, or already moved",
                "push into one mutable initialized available Vec local",
            );
            return None;
        }
        let reserved_actions = self.preflight_push_cleanup(value, at)?;
        if !reserve_owned_commit_transition(&mut self.cfg, at, self.errors) {
            return None;
        }
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            release_owned_commit_transition(&mut self.cfg);
            return None;
        }
        let Some(value) = self.value(value, self.element) else {
            self.release_cleanup_capacity(reserved_actions);
            release_owned_commit_transition(&mut self.cfg);
            return None;
        };
        let consumed = self.owners.owner(value);
        self.release_cleanup_capacity(reserved_actions);
        release_owned_commit_transition(&mut self.cfg);
        let cleanup = self.push_instruction_cleanup(at, None)?;
        if !self.emit_effect(
            at,
            raw::InstructionKind::VecPush { vector: binding.place, value, cleanup },
        ) {
            return None;
        }
        if let Some(owner) = consumed
            && let Some(delta) = self.owners.consume_owner(owner)
        {
            apply_owner_delta(&mut self.known_string_bytes, delta);
        }
        Some(())
    }

    fn target_consumption_span(
        &self,
        id: u32,
        target: raw::PlaceId,
        consumes_reference: bool,
    ) -> Option<UntrustedSpan> {
        let expression = self.expression(id)?;
        match &expression.kind {
            RawExpressionKind::Reference { name }
                if consumes_reference
                    && self.bindings.get(&name.text).is_some_and(|binding| {
                        binding.place == target && self.owners.contains(binding.place)
                    }) =>
            {
                Some(name.span)
            }
            RawExpressionKind::Clone { value, .. } => {
                self.target_consumption_span(*value, target, false)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                arguments
                    .iter()
                    .find_map(|argument| self.target_consumption_span(*argument, target, false))
            }
            RawExpressionKind::Call { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.target_consumption_span(*argument, target, true)),
            RawExpressionKind::VecConstruction { elements, .. } => elements
                .iter()
                .find_map(|element| self.target_consumption_span(*element, target, true)),
            _ => None,
        }
    }

    pub(in crate::data_ownership_v1) fn lower_root_local(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        after_control_flow: bool,
    ) -> Option<()> {
        if after_control_flow {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "owned Vec control flow must immediately precede the final return",
                "move every outer declaration before the single top-level control-flow statement",
            );
            return None;
        }
        self.lower_local(statement)
    }

    pub(in crate::data_ownership_v1) fn lower_root_assignment(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        target: u32,
        value: u32,
        after_control_flow: bool,
    ) -> Option<()> {
        if after_control_flow {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "owned Vec control-flow lowering excludes assignment after its exit",
                "leave the joined outer owned state unchanged and return it directly",
            );
            return None;
        }
        let target_expression = self.expression(target)?.clone();
        let RawExpressionKind::Reference { name } = target_expression.kind else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), target_expression.span),
                "owned assignment requires one root local target",
                "assign only to an initialized mutable String or exact Vec local",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("owned assignment target '{}' is not declared", name.text),
                "assign one exact preceding local",
            );
            return None;
        };
        if binding.ty.is_copy()
            || !matches!(binding.ty.category, TypeCategory::String | TypeCategory::Vec)
            || (binding.ty.category == TypeCategory::Vec && binding.ty != self.vec_ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), name.span),
                "assignment target is outside the exact supported owned type",
                "assign only to String or the function's exact Vec type",
            );
            return None;
        }
        if !binding.mutable || !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "owned assignment target is immutable, uninitialized, or already moved",
                "assign only to an initialized mutable available owned local",
            );
            return None;
        }
        if let Some(reference_span) = self.target_consumption_span(value, binding.place, true) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), reference_span),
                "owned assignment cannot consume its destination while preparing its replacement",
                "prepare a distinct owned value before replacement",
            );
            return None;
        }
        let assignment_span = span(self.input.sources(), statement.span);
        if !reserve_owned_commit_transition(&mut self.cfg, assignment_span, self.errors) {
            return None;
        }
        let Some(prepared) = self.value(value, binding.ty) else {
            release_owned_commit_transition(&mut self.cfg);
            return None;
        };
        release_owned_commit_transition(&mut self.cfg);
        if !self.emit_effect(
            assignment_span,
            raw::InstructionKind::ReplacePlace { place: binding.place, value: prepared },
        ) {
            return None;
        }
        if !self.replace_owner(prepared, binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), statement.span),
                "owned assignment replacement has no distinct prepared owner",
                "replace from one available independently prepared owned value",
            );
            return None;
        }
        Some(())
    }

    pub(in crate::data_ownership_v1) fn lower_root_push_effect(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        expression: u32,
        after_control_flow: bool,
    ) -> Option<()> {
        if after_control_flow {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "owned Vec control-flow lowering excludes effects after its exit",
                "leave the joined outer owned state unchanged and return it directly",
            );
            return None;
        }
        self.lower_push_effect(expression, None)
    }
}
