use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::function_catalog::{FunctionResolution, FunctionSignature};
use super::super::global_resource_limits::checked_string_concat_bytes;
use super::super::owned_lowering_resources::{
    OwnedStringPreparationBudget, preflight_owned_string_preparation,
};
use super::super::owner_state::{OwnerDelta, apply_owner_delta};
use super::super::span;
use super::super::string_vec_resource_estimates::{
    OwnedStringEstimateError, OwnedStringPreparationEstimate, estimate_owned_string_call_arguments,
};
use super::PrivateStringLowerer;

impl PrivateStringLowerer<'_, '_, '_> {
    fn readable_reference(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
    ) -> Option<(raw::PlaceId, Option<u64>)> {
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("binding '{}' is not declared in this function", name.text),
                "reference one exact preceding String local",
            );
            return None;
        };
        if binding.ty != self.ty || !self.owners.contains(binding.place) {
            self.errors.at(
                "ZRYNA-M3011",
                span(self.input.sources(), name.span),
                format!("String binding '{}' was already moved", name.text),
                "use or move each owned String binding only while it remains available",
            );
            return None;
        }
        Some((binding.place, self.known_bytes.get(&binding.place).copied().flatten()))
    }

    fn place_for_read(&mut self, id: u32) -> Option<(raw::PlaceId, Option<u64>)> {
        let expression = self.expression(id)?.clone();
        if let RawExpressionKind::Reference { name } = expression.kind {
            self.readable_reference(&name)
        } else {
            let (_, owner) = self.value(id)?;
            Some((owner, self.known_bytes.get(&owner).copied().flatten()))
        }
    }

    pub(in crate::data_ownership_v1) fn value(
        &mut self,
        id: u32,
    ) -> Option<(raw::ValueId, raw::PlaceId)> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        if !self.preflight_string_expression(id, at) {
            return None;
        }
        match expression.kind {
            RawExpressionKind::StringLiteral { spelling } => {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let value = self
                    .push_temporary(at, raw::InstructionKind::StringFromUtf8 { bytes, cleanup })?;
                self.known_bytes.insert(
                    value.1,
                    Some(u64::try_from(spelling.len().saturating_sub(2)).unwrap_or(u64::MAX)),
                );
                Some(value)
            }
            RawExpressionKind::Reference { name } => {
                let (source, _) = self.readable_reference(&name)?;
                let value =
                    self.push_temporary(at, raw::InstructionKind::MoveFromPlace { place: source })?;
                let delta = self
                    .owners
                    .rehome_move_result(value.0, source)
                    .expect("readable String move has one registered result owner");
                apply_owner_delta(&mut self.known_bytes, delta);
                Some(value)
            }
            RawExpressionKind::Clone { value, .. } => {
                let (source, bytes) = self.place_for_read(value)?;
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let cloned = self.push_temporary(
                    at,
                    raw::InstructionKind::StringClone { place: source, cleanup },
                )?;
                self.known_bytes.insert(cloned.1, bytes);
                Some(cloned)
            }
            RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
                let [left, right] = arguments.as_slice() else {
                    self.errors.at(
                        "ZRYNA-M3012",
                        span(self.input.sources(), callee.span),
                        "String concat requires exactly two operands",
                        "call concat(left, right) with two available String values",
                    );
                    return None;
                };
                let (left, left_bytes) = self.place_for_read(*left)?;
                let (right, right_bytes) = self.place_for_read(*right)?;
                let bytes = match (left_bytes, right_bytes) {
                    (Some(left), Some(right)) => {
                        let Some(bytes) = checked_string_concat_bytes(left, right) else {
                            self.errors.at(
                                "ZRYNA-M3012",
                                at,
                                "String concatenation exceeds the sealed runtime byte limit",
                                "reduce the statically known concatenated String size",
                            );
                            return None;
                        };
                        Some(bytes)
                    }
                    _ => None,
                };
                let cleanup = self.push_instruction_cleanup(at, None)?;
                let concatenated = self.push_temporary(
                    at,
                    raw::InstructionKind::StringConcat { left, right, cleanup },
                )?;
                self.known_bytes.insert(concatenated.1, bytes);
                Some(concatenated)
            }
            RawExpressionKind::Call { callee, arguments, .. } => {
                self.direct_call(&callee, &arguments, at)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "this private String expression is outside straight-line move lowering",
                    "use a String literal or move one preceding typed String local",
                );
                None
            }
        }
    }

    fn resolve_owned_callee(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
    ) -> Option<FunctionSignature> {
        let signature = match self.catalog.resolve(self.module, &callee.text) {
            FunctionResolution::Exact(signature) => signature.clone(),
            FunctionResolution::WrongCase => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("call name '{}' has the wrong portable ASCII case", callee.text),
                    "use the callee's exact declared spelling",
                );
                return None;
            }
            FunctionResolution::Missing => {
                self.errors.at(
                    "ZRYNA-M3002",
                    span(self.input.sources(), callee.span),
                    format!("function '{}' is not declared in this module", callee.text),
                    "call one exact private same-module function",
                );
                return None;
            }
        };
        if !signature.private {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned calls require one private same-module callee",
                "keep String producers and identity functions internal",
            );
            return None;
        }
        if signature.result != self.ty
            || signature.has_borrow_parameters()
            || signature.parameters.len() > 1
            || signature.parameters.iter().any(|parameter| *parameter != self.ty)
        {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned call signature is outside the sealed String producer/identity checkpoint",
                "call a private zero-argument String producer or one-String identity function",
            );
            return None;
        }
        Some(signature)
    }

    fn prepare_direct_call_arguments(
        &mut self,
        arguments: &[u32],
        at: Span,
    ) -> Option<(Vec<raw::CallArgument>, Vec<raw::PlaceId>)> {
        let mut lowered = Vec::with_capacity(arguments.len());
        let mut owners = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let (value, owner) = self.value(*argument)?;
            if !self.owners.contains(owner) {
                self.errors.at(
                    "ZRYNA-M3011",
                    at,
                    "owned call argument has no available String owner",
                    "pass each String value exactly once",
                );
                return None;
            }
            owners.push(owner);
            lowered.push(raw::CallArgument::Value(value));
        }
        Some((lowered, owners))
    }

    fn release_direct_call_commit(&mut self) {
        self.cfg.release_transitions(1);
        self.release_local_place();
        self.cfg.release_values(1);
    }

    fn preflight_direct_call_preparation(
        &mut self,
        arguments: &[u32],
        at: Span,
    ) -> Option<OwnedStringPreparationEstimate> {
        let estimate = match estimate_owned_string_call_arguments(
            self.function,
            &self.bindings,
            &self.owners,
            self.ty,
            arguments,
            self.owners.pending().len(),
        ) {
            Ok(estimate) => estimate,
            Err(OwnedStringEstimateError::Unsupported) => {
                self.errors.at(
                    "ZRYNA-M3012",
                    at,
                    "owned String call argument is outside checked recursive preparation",
                    "use an admitted String literal, move, clone, concat, or private String call",
                );
                return None;
            }
            Err(OwnedStringEstimateError::Unavailable(reference)) => {
                self.errors.at(
                    "ZRYNA-M3011",
                    span(self.input.sources(), reference),
                    "owned call argument has no available String owner",
                    "pass each String value exactly once",
                );
                return None;
            }
            Err(OwnedStringEstimateError::Overflow) => {
                self.errors.at(
                    "ZRYNA-M3201",
                    at,
                    "recursive owned String call preparation overflows its checked resource estimate",
                    "reduce nested String-producing call arguments",
                );
                return None;
            }
        };
        preflight_owned_string_preparation(
            estimate,
            OwnedStringPreparationBudget {
                cleanup_plans: self.cleanup_plans.len(),
                reserved_cleanup_plans: self.reserved_cleanup_plans,
                cleanup_actions: self.cleanup_actions,
                reserved_cleanup_actions: self.reserved_cleanup_actions,
                places: self.places.len(),
                reserved_places: self.reserved_places,
            },
            &mut self.cfg,
            at,
            self.errors,
        )
        .then_some(estimate)
    }

    pub(in crate::data_ownership_v1) fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        at: Span,
    ) -> Option<(raw::ValueId, raw::PlaceId)> {
        let signature = self.resolve_owned_callee(callee)?;
        if arguments.len() != signature.parameters.len() {
            self.errors.at(
                "ZRYNA-M3012",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameters.len()
                ),
                "pass the exact declared String argument",
            );
            return None;
        }
        let estimate = self.preflight_direct_call_preparation(arguments, at)?;
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
        let reserved_actions = estimate.root_cleanup_actions.expect("direct call cleanup estimate");
        if !self.reserve_cleanup_capacity(reserved_actions, at) {
            self.release_direct_call_commit();
            return None;
        }
        let prepared = self.prepare_direct_call_arguments(arguments, at);
        let Some((lowered, owners)) = prepared else {
            self.release_cleanup_capacity(reserved_actions);
            self.release_direct_call_commit();
            return None;
        };
        let cleanup = raw::CleanupPlanId(
            u32::try_from(self.cleanup_plans.len()).expect("cleanup reservation bounds plan id"),
        );
        for (argument, owner) in lowered.iter().zip(owners) {
            let raw::CallArgument::Value(value) = argument else {
                unreachable!("private String calls use only by-value arguments");
            };
            let Some(delta) = self.owners.transfer(*value) else {
                self.errors.at(
                    "ZRYNA-M3011",
                    at,
                    "owned call argument has no unique available String owner",
                    "pass each String value exactly once",
                );
                self.release_cleanup_capacity(reserved_actions);
                self.release_direct_call_commit();
                return None;
            };
            debug_assert_eq!(delta, OwnerDelta::Transferred { owner });
            apply_owner_delta(&mut self.known_bytes, delta);
        }
        self.release_cleanup_capacity(reserved_actions);
        self.release_direct_call_commit();
        let committed_cleanup = self.push_instruction_cleanup(at, None)?;
        debug_assert_eq!(committed_cleanup, cleanup);
        let result = self.push_temporary(
            at,
            raw::InstructionKind::DirectCall {
                callee: signature.id,
                arguments: lowered,
                cleanup: committed_cleanup,
            },
        )?;
        self.known_bytes.insert(result.1, None);
        Some(result)
    }
}
