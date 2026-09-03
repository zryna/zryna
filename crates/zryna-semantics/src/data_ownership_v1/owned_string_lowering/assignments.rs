use zryna_ir::data_ownership_v1::raw;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawStatementKind};

use super::super::owned_cfg_state::{
    release_owned_commit_transition, reserve_owned_commit_transition,
};
use super::super::owner_state::{OwnerDelta, apply_owner_delta};
use super::super::type_model::Binding;
use super::super::{semantic_type, span};
use super::{PrivateStringLowerer, StringBranchTypes};

impl PrivateStringLowerer<'_, '_, '_> {
    pub(super) fn lower_string_local(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        let local_ty = semantic_type(
            types.file,
            *type_syntax,
            self.module,
            types.declarations,
            types.graph,
            types.node_types,
            self.errors,
        )?;
        if local_ty != self.ty {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), statement.span),
                "private String lowering requires exact typed String locals",
                "declare each owned local as String",
            );
            return None;
        }
        if self.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text)) {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let at = span(self.input.sources(), statement.span);
        if !self.reserve_local_commit(at) {
            return None;
        }
        let Some((value, temporary)) = self.value(*initializer) else {
            self.release_local_commit();
            return None;
        };
        let local = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let initialize = raw::Instruction {
            result: None,
            span: at,
            kind: raw::InstructionKind::InitializePlace { place: local, value },
        };
        self.release_local_commit();
        self.places.push(raw::Place {
            id: local,
            ty: self.ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local = self.next_local.checked_add(1)?;
        if !self.cfg.emit(initialize, self.errors) {
            return None;
        }
        let Some(delta) = self.owners.rename(value, local) else {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "String local initializer has no available owner",
                "initialize the local from one available String value",
            );
            return None;
        };
        debug_assert_eq!(delta, OwnerDelta::Renamed { from: temporary, to: local });
        apply_owner_delta(&mut self.known_bytes, delta);
        self.bindings
            .insert(name.text.clone(), Binding { ty: self.ty, place: local, mutable: *mutable });
        Some(())
    }

    pub(super) fn lower_root_local(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        after_control_flow: bool,
        types: StringBranchTypes<'_>,
    ) -> Option<()> {
        let RawStatementKind::LocalDeclaration { mutable, name, type_syntax, initializer, .. } =
            &statement.kind
        else {
            return None;
        };
        if after_control_flow {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "owned String control flow must immediately precede the final return",
                "move every outer declaration before the single top-level control-flow statement",
            );
            return None;
        }
        let local_ty = semantic_type(
            types.file,
            *type_syntax,
            self.module,
            types.declarations,
            types.graph,
            types.node_types,
            self.errors,
        )?;
        if local_ty != self.ty {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), statement.span),
                "private String lowering requires exact typed String locals",
                "declare each local as String",
            );
            return None;
        }
        if self.bindings.keys().any(|existing| existing.eq_ignore_ascii_case(&name.text)) {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' collides under portable ASCII case folding", name.text),
                "give every binding one portable case-insensitive unique name",
            );
            return None;
        }
        let local_span = span(self.input.sources(), statement.span);
        if !self.reserve_local_commit(local_span) {
            return None;
        }
        let Some((value, temporary)) = self.value(*initializer) else {
            self.release_local_commit();
            return None;
        };
        let local = raw::PlaceId(u32::try_from(self.places.len()).expect("bounded places"));
        let initialize = raw::Instruction {
            result: None,
            span: local_span,
            kind: raw::InstructionKind::InitializePlace { place: local, value },
        };
        self.release_local_commit();
        self.places.push(raw::Place {
            id: local,
            ty: self.ty.ir,
            span: local_span,
            kind: raw::PlaceKind::Local(self.next_local),
        });
        self.next_local += 1;
        if !self.cfg.emit(initialize, self.errors) {
            return None;
        }
        let Some(delta) = self.owners.rename(value, local) else {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), statement.span),
                "String local initializer has no available owner",
                "initialize the local from one available String value",
            );
            return None;
        };
        debug_assert_eq!(delta, OwnerDelta::Renamed { from: temporary, to: local });
        apply_owner_delta(&mut self.known_bytes, delta);
        self.bindings
            .insert(name.text.clone(), Binding { ty: self.ty, place: local, mutable: *mutable });
        Some(())
    }

    pub(super) fn lower_root_assignment(
        &mut self,
        statement: &syntax::RawStatementSyntax,
        after_control_flow: bool,
    ) -> Option<()> {
        let RawStatementKind::Assignment { target, value, .. } = &statement.kind else {
            return None;
        };
        if after_control_flow {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), statement.span),
                "owned String control-flow lowering excludes assignment after its exit",
                "leave the joined outer String state unchanged and return it directly",
            );
            return None;
        }
        let target_expression = self.expression(*target)?.clone();
        let RawExpressionKind::Reference { name } = target_expression.kind else {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), target_expression.span),
                "String assignment requires one root local target",
                "assign only to an initialized mutable String local",
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("String assignment target '{}' is not declared", name.text),
                "assign one exact preceding String local",
            );
            return None;
        };
        if binding.ty != self.ty {
            self.errors.at(
                "ZRYNA-M3012",
                span(self.input.sources(), name.span),
                "String assignment target has the wrong exact type",
                "assign only to an exact String local",
            );
            return None;
        }
        if !binding.mutable || !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "String assignment target is immutable, uninitialized, or already moved",
                "assign only to an initialized mutable available String local",
            );
            return None;
        }
        if let Some(reference_span) = self.target_consumption_span(*value, binding.place, true) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), reference_span),
                "String assignment cannot consume its destination while preparing its replacement",
                "prepare a distinct String value or explicitly clone the destination",
            );
            return None;
        }
        let assignment_span = span(self.input.sources(), statement.span);
        if !reserve_owned_commit_transition(&mut self.cfg, assignment_span, self.errors) {
            return None;
        }
        let Some((prepared_value, prepared_owner)) = self.value(*value) else {
            release_owned_commit_transition(&mut self.cfg);
            return None;
        };
        release_owned_commit_transition(&mut self.cfg);
        if !self.cfg.emit(
            raw::Instruction {
                result: None,
                span: assignment_span,
                kind: raw::InstructionKind::ReplacePlace {
                    place: binding.place,
                    value: prepared_value,
                },
            },
            self.errors,
        ) {
            return None;
        }
        let Some(delta) = self.owners.replace(prepared_value, binding.place) else {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), statement.span),
                "String assignment replacement has no distinct prepared owner",
                "replace from one available independently prepared String value",
            );
            return None;
        };
        debug_assert_eq!(
            delta,
            OwnerDelta::Replaced { prepared: prepared_owner, target: binding.place }
        );
        apply_owner_delta(&mut self.known_bytes, delta);
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
            _ => None,
        }
    }
}
