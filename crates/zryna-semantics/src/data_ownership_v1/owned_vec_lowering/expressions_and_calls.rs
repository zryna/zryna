use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::owner_state::{OwnerDelta, apply_owner_delta};
use super::super::span;
use super::super::string_vec_resource_estimates::cleanup_actions_after_transfer;
use super::super::type_model::Ty;
use super::PrivateVecLowerer;

impl PrivateVecLowerer<'_, '_, '_> {
    fn string_place_for_read(&mut self, id: u32) -> Option<(raw::PlaceId, u64)> {
        let expression = self.expression(id)?.clone();
        if let RawExpressionKind::Reference { name } = expression.kind {
            super::super::owned_string_read::local(
                &name,
                &self.bindings,
                &self.owners,
                &self.known_string_bytes,
                span(self.input.sources(), name.span),
                self.errors,
            )
        } else {
            let string = self
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == TypeCategory::String)
                .copied()?;
            let value = self.value(id, string)?;
            let owner = self.owners.owner(value)?;
            Some((owner, *self.known_string_bytes.get(&owner)?))
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(in crate::data_ownership_v1) fn value(
        &mut self,
        id: u32,
        expected: Ty,
    ) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if expected.category == TypeCategory::String
            && !self.preflight_string_expression(id, expected, at)
        {
            return None;
        }
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } if expected.category == TypeCategory::Bool => {
                Some(self.emit(expected, at, raw::InstructionKind::BoolLiteral(value))?.0)
            }
            RawExpressionKind::I32Literal { spelling }
                if expected.category == TypeCategory::I32 =>
            {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3013",
                        at,
                        "Vec element integer is outside i32",
                        "use an in-range i32 element",
                    );
                    None
                })?;
                Some(self.emit(expected, at, raw::InstructionKind::I32Literal(value))?.0)
            }
            RawExpressionKind::StringLiteral { spelling }
                if expected.category == TypeCategory::String =>
            {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let byte_count = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringFromUtf8 { bytes, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, byte_count);
                Some(value)
            }
            RawExpressionKind::Reference { name } => {
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), name.span),
                        format!("Vec operand '{}' is not declared", name.text),
                        "reference one exact preceding typed local",
                    );
                    return None;
                };
                if binding.ty != expected {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), name.span),
                        "Vec operand has the wrong exact element or container type",
                        "use the exact declared Vec element type",
                    );
                    return None;
                }
                if expected.is_copy() {
                    Some(
                        self.emit(
                            expected,
                            at,
                            raw::InstructionKind::CopyFromPlace { place: binding.place },
                        )?
                        .0,
                    )
                } else {
                    if !self.owners.contains(binding.place) {
                        self.errors.at(
                            "ZRYNA-M3014",
                            span(self.input.sources(), name.span),
                            format!("owned value '{}' was already moved", name.text),
                            "move each owned value at most once",
                        );
                        return None;
                    }
                    let (value, owner) = self.emit(
                        expected,
                        at,
                        raw::InstructionKind::MoveFromPlace { place: binding.place },
                    )?;
                    let owner = owner?;
                    let delta = self.owners.rehome_move_result(value, binding.place)?;
                    debug_assert_eq!(delta, OwnerDelta::Renamed { from: binding.place, to: owner });
                    apply_owner_delta(&mut self.known_string_bytes, delta);
                    Some(value)
                }
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::String => {
                let (source, bytes) = self.string_place_for_read(value)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringClone { place: source, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, bytes);
                Some(value)
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::Vec => {
                self.clone_vec(value, expected, at)
            }
            RawExpressionKind::Call { callee, arguments, .. }
                if expected.category == TypeCategory::String && callee.text == "concat" =>
            {
                let [left, right] = super::super::owned_string_read::concat_arguments(
                    &arguments,
                    span(self.input.sources(), callee.span),
                    self.errors,
                )?;
                let (left, left_bytes) = self.string_place_for_read(left)?;
                let (right, right_bytes) = self.string_place_for_read(right)?;
                let bytes = super::super::owned_string_read::concat_bytes(
                    left_bytes,
                    right_bytes,
                    at,
                    self.errors,
                )?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let (value, owner) = self.emit(
                    expected,
                    at,
                    raw::InstructionKind::StringConcat { left, right, cleanup },
                )?;
                self.known_string_bytes.insert(owner?, bytes);
                Some(value)
            }
            RawExpressionKind::Call { callee, arguments, .. }
                if expected.category == TypeCategory::Vec =>
            {
                self.direct_call(&callee, &arguments, expected, at)
            }
            RawExpressionKind::VecConstruction { type_syntax, elements, .. }
                if expected.category == TypeCategory::Vec =>
            {
                self.construct_vec(type_syntax, &elements, expected, at)
            }
            RawExpressionKind::Index { base, index, .. } => {
                if expected != self.element || !expected.is_copy() {
                    self.errors.at(
                        "ZRYNA-M3013",
                        at,
                        "Vec indexing is admitted only for the exact Copy element type",
                        "index Vec<bool> or Vec<i32> and return that exact scalar type",
                    );
                    return None;
                }
                let base_expression = self.expression(base)?.clone();
                let RawExpressionKind::Reference { name } = base_expression.kind else {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), base_expression.span),
                        "Vec indexing requires an addressable local Vec",
                        "index one initialized Vec local",
                    );
                    return None;
                };
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("Vec binding '{}' is not declared in this function", name.text),
                        "reference one exact preceding Vec local",
                    );
                    return None;
                };
                if binding.ty.category != TypeCategory::Vec || !self.owners.contains(binding.place)
                {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), name.span),
                        "indexed Vec is unavailable or already moved",
                        "index one initialized available Vec local",
                    );
                    return None;
                }
                let i32_ty = self
                    .node_types
                    .iter()
                    .flatten()
                    .find(|ty| ty.category == TypeCategory::I32)
                    .copied()?;
                let index = self.value(index, i32_ty)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                Some(
                    self.emit(
                        expected,
                        at,
                        raw::InstructionKind::VecIndexCopy { place: binding.place, index, cleanup },
                    )?
                    .0,
                )
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3013",
                    at,
                    "expression is outside private straight-line Vec lowering",
                    "use exact bool, i32, or String elements and private Vec moves",
                );
                None
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(in crate::data_ownership_v1) fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let signature = self.resolve_owned_callee(callee, expected)?;
        if arguments.len() != signature.parameters.len() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                "pass the exact declared Vec argument",
            );
            return None;
        }
        let diagnostics_before_preflight = self.errors.len();
        let preparation = arguments.first().and_then(|argument| {
            self.estimate_vec_preparation(*argument, self.vec_ty, self.owners.pending().len(), at)
        });
        if self.errors.len() != diagnostics_before_preflight {
            return None;
        }
        let moves_existing_owner = arguments.first().is_some_and(|argument| {
            self.expression(*argument).is_some_and(|expression| {
                matches!(&expression.kind, RawExpressionKind::Reference { name }
                if self.bindings.get(&name.text).is_some_and(|binding| {
                    binding.ty == self.vec_ty && self.owners.contains(binding.place)
                }))
            })
        });
        let reserved_actions = if let Some(preparation) = preparation {
            let Some(actions) = preparation.end_pending.checked_sub(arguments.len()) else {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "Vec call preparation underflows its checked owner estimate",
                    "reduce nested owned Vec call arguments",
                );
                return None;
            };
            if !self.preflight_string_sequence_with_enclosing_cleanup(
                preparation.resources,
                actions,
                at,
            ) {
                return None;
            }
            actions
        } else {
            cleanup_actions_after_transfer(self.owners.pending().len(), moves_existing_owner)
        };
        self.cfg.reserve_values(1, at, self.errors)?;
        if !self.reserve_local_place(at) {
            self.cfg.release_values(1);
            return None;
        }
        if self.cfg.reserve_transitions(1, at, self.errors).is_none() {
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        }
        let prepared = (|| {
            let mut lowered = Vec::with_capacity(arguments.len());
            let mut transferred = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let value = self.value(*argument, self.vec_ty)?;
                let Some(owner) = self.owners.owner(value) else {
                    self.errors.at(
                        "ZRYNA-M3014",
                        at,
                        "owned Vec call argument has no available owner",
                        "pass each Vec value exactly once",
                    );
                    return None;
                };
                transferred.push((value, owner));
                lowered.push(raw::CallArgument::Value(value));
            }
            Some((lowered, transferred))
        })();
        let Some((lowered, transferred)) = prepared else {
            self.release_cleanup_capacity(reserved_actions);
            self.cfg.release_transitions(1);
            self.release_local_place();
            self.cfg.release_values(1);
            return None;
        };
        let cleanup = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        for (value, owner) in transferred {
            if !self.transfer_owner(value) {
                self.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "owned Vec call argument has no unique available owner",
                    "pass each Vec value exactly once",
                );
                self.release_cleanup_capacity(reserved_actions);
                self.cfg.release_transitions(1);
                self.release_local_place();
                self.cfg.release_values(1);
                return None;
            }
            debug_assert!(!self.owners.contains(owner));
        }
        self.release_cleanup_capacity(reserved_actions);
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
        let committed_cleanup = self.push_cleanup(at, None)?;
        debug_assert_eq!(committed_cleanup, cleanup);
        Some(
            self.emit(
                expected,
                at,
                raw::InstructionKind::DirectCall {
                    callee: signature.id,
                    arguments: lowered,
                    cleanup: committed_cleanup,
                },
            )?
            .0,
        )
    }
}
