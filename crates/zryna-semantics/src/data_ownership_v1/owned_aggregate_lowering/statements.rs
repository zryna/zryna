use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{RawExpressionKind, RawStatementKind, RawStatementSyntax};

use super::super::layout_graph::semantic_type;
use super::super::span;
use super::super::type_model::{Binding, ProjectedAggregateMoveContext, Ty};
use super::PrivateOwnedAggregateLowerer;

pub(super) enum StatementOutcome {
    Continue,
    Return(raw::ValueId, Span),
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn lower_statement(
        &mut self,
        statement_id: u32,
        statement: &RawStatementSyntax,
        result: Ty,
        final_statement: Option<u32>,
        return_count: usize,
    ) -> Option<StatementOutcome> {
        match &statement.kind {
            RawStatementKind::LocalDeclaration {
                mutable, name, type_syntax, initializer, ..
            } => {
                if self.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text)) {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!(
                            "binding '{}' collides under portable ASCII case folding",
                            name.text
                        ),
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
                if !matches!(
                    self.local_preparation_route(ty),
                    super::mixed_shape::PreparationRoute::Aggregate
                        | super::mixed_shape::PreparationRoute::MixedSummary
                ) {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), statement.span),
                        "local type is outside the private owned aggregate graph",
                        "use bool, i32, String, or a supported Struct/Enum/FixedArray type",
                    );
                    return None;
                }
                let statement_span = span(self.input.sources(), statement.span);
                if let Some(source) = self.partial_local_transfer_source(*initializer, ty) {
                    let place = self.lower_partial_local_transfer(source, ty, statement_span)?;
                    self.bindings
                        .insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
                    return Some(StatementOutcome::Continue);
                }
                let aggregate_projection_local =
                    matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        && self.expression(*initializer).is_some_and(|expression| {
                            matches!(
                                &expression.kind,
                                RawExpressionKind::FieldAccess { .. }
                                    | RawExpressionKind::Index { .. }
                            )
                        });
                let projected_aggregate_clone_operand =
                    matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        .then(|| self.expression(*initializer))
                        .flatten()
                        .and_then(|expression| match &expression.kind {
                            RawExpressionKind::Clone { value, .. }
                                if self.expression(*value).is_some_and(|operand| {
                                    matches!(
                                        &operand.kind,
                                        RawExpressionKind::FieldAccess { .. }
                                            | RawExpressionKind::Index { .. }
                                    )
                                }) =>
                            {
                                Some((*value, span(self.input.sources(), expression.span)))
                            }
                            _ => None,
                        });
                let value = if let Some((operand, clone_span)) = projected_aggregate_clone_operand {
                    self.clone_projected_aggregate_local(operand, ty, clone_span)?
                } else if aggregate_projection_local {
                    self.projected_value(
                        *initializer,
                        ty,
                        Some(ProjectedAggregateMoveContext::DirectLocal),
                    )?
                } else {
                    super::constructor_preparation::PreparedValue::prepare_local(
                        self,
                        *initializer,
                        ty,
                    )?
                    .consume()
                };
                if self.budget_places() >= ir::MAX_PLACES_PER_FUNCTION {
                    self.errors.at(
                        "ZRYNA-M3201",
                        span(self.input.sources(), statement.span),
                        "derived aggregate places exceed the per-function M3 limit",
                        "reduce private aggregate locals",
                    );
                    return None;
                }
                let place = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
                self.places.push(raw::Place {
                    id: place,
                    ty: ty.ir,
                    span: statement_span,
                    kind: raw::PlaceKind::Local(self.next_local),
                });
                self.next_local += 1;
                if !self.emit_effect(
                    statement_span,
                    raw::InstructionKind::InitializePlace { place, value },
                ) {
                    return None;
                }
                if !ty.is_copy()
                    && self
                        .owners
                        .rename(value, place)
                        .map(|delta| self.preparation_facts.apply(delta))
                        .is_none()
                {
                    self.errors.at(
                        "ZRYNA-M3014",
                        statement_span,
                        "owned aggregate local initializer has no available owner",
                        "initialize from one exact available owned value",
                    );
                    return None;
                }
                self.bindings.insert(name.text.clone(), Binding { ty, place, mutable: *mutable });
            }
            RawStatementKind::Return { value, .. } => {
                let return_span = span(self.input.sources(), statement.span);
                let aggregate_projection_return =
                    matches!(result.category, TypeCategory::Struct | TypeCategory::FixedArray)
                        && self.expression(*value).is_some_and(|expression| {
                            matches!(
                                expression.kind,
                                RawExpressionKind::FieldAccess { .. }
                                    | RawExpressionKind::Index { .. }
                            )
                        });
                let value = if let Some(source) =
                    self.partial_return_transfer_source(*value, result)
                {
                    self.lower_partial_return_transfer(source, result, return_span)?
                } else if aggregate_projection_return {
                    if !self.function.parameters.is_empty()
                        || Some(statement_id) != final_statement
                        || return_count != 1
                    {
                        self.errors.at(
                                "ZRYNA-M3016",
                                return_span,
                                "direct aggregate-subobject return requires the sole final return of one parameter-free private function",
                                "return one complete static Struct or fixed-array subobject from a local root as the sole final return in a parameter-free function",
                            );
                        return None;
                    }
                    self.projected_value(
                        *value,
                        result,
                        Some(ProjectedAggregateMoveContext::FinalReturn),
                    )?
                } else {
                    self.value(*value, result)?
                };
                return Some(StatementOutcome::Return(value, return_span));
            }
            RawStatementKind::Assignment { target, value, .. } => {
                let target_expression = self.expression(*target)?.clone();
                if !matches!(
                    target_expression.kind,
                    RawExpressionKind::Reference { .. }
                        | RawExpressionKind::FieldAccess { .. }
                        | RawExpressionKind::Index { .. }
                ) {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), target_expression.span),
                        "owned aggregate assignment target is not an addressable static place",
                        "assign to one mutable root or static Struct/FixedArray String projection",
                    );
                    return None;
                }
                if !matches!(target_expression.kind, RawExpressionKind::Reference { .. }) {
                    let target_ty = self.projection_expression_type(*target);
                    if target_ty.is_some_and(|ty| {
                        matches!(ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
                    }) {
                        self.lower_projected_aggregate_assignment(
                            *target,
                            *value,
                            span(self.input.sources(), statement.span),
                        )?;
                        return Some(StatementOutcome::Continue);
                    }
                    let target_place = self.owned_place(*target)?;
                    let target_span = span(self.input.sources(), target_expression.span);
                    if target_place.is_root || target_place.ty.category != TypeCategory::String {
                        self.errors.at(
                            "ZRYNA-M3013",
                            target_span,
                            "owned projected assignment requires one exact String leaf",
                            "assign only to a static String field or constant String fixed-array element",
                        );
                        return None;
                    }
                    if !target_place.mutable
                        || !self.projection_available(target_place.place, target_place.root)
                    {
                        self.errors.at(
                            "ZRYNA-M3014",
                            target_span,
                            "owned projected assignment target is immutable, moved, or overlaps a moved subobject",
                            "assign only to an initialized mutable available String projection",
                        );
                        return None;
                    }
                    if let Some(reference_span) =
                        self.target_consumption_span(*value, target_place.root, true)
                    {
                        self.errors.at(
                            "ZRYNA-M3014",
                            span(self.input.sources(), reference_span),
                            "owned projected assignment cannot consume its enclosing root while preparing the replacement",
                            "prepare a distinct String value before replacing the projection",
                        );
                        return None;
                    }
                    let assignment_span = span(self.input.sources(), statement.span);
                    if !self.reserve_transition(assignment_span) {
                        return None;
                    }
                    let Some(prepared) = self.value(*value, target_place.ty) else {
                        self.release_transition();
                        return None;
                    };
                    self.release_transition();
                    if !self.emit_effect(
                        assignment_span,
                        raw::InstructionKind::ReplacePlace {
                            place: target_place.place,
                            value: prepared,
                        },
                    ) {
                        return None;
                    }
                    if self
                        .owners
                        .transfer(prepared)
                        .map(|delta| self.preparation_facts.apply(delta))
                        .is_none()
                    {
                        self.errors.at(
                            "ZRYNA-M3014",
                            assignment_span,
                            "owned projected assignment replacement has no distinct prepared owner",
                            "replace from one available independently prepared String value",
                        );
                        return None;
                    }
                    return Some(StatementOutcome::Continue);
                }
                let RawExpressionKind::Reference { name } = target_expression.kind else {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), target_expression.span),
                        "owned aggregate assignment requires one root local target",
                        "assign only to an initialized mutable Struct, Enum, or fixed-array local",
                    );
                    return None;
                };
                let Some(binding) = self.bindings.get(&name.text).cloned() else {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("aggregate assignment target '{}' is not declared", name.text),
                        "assign one exact preceding local",
                    );
                    return None;
                };
                if binding.ty.is_copy()
                    || !matches!(
                        binding.ty.category,
                        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
                    )
                    || !self.supported(binding.ty)
                {
                    self.errors.at(
                        "ZRYNA-M3013",
                        span(self.input.sources(), name.span),
                        "assignment target is outside the exact supported owned aggregate graph",
                        "assign only to a supported String-bearing Struct, Enum, or fixed-array root",
                    );
                    return None;
                }
                if !binding.mutable || !self.whole_root_available(binding.place) {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), name.span),
                        "owned aggregate assignment target is immutable, moved, or only partially available",
                        "assign only to an initialized mutable aggregate root before moving any projection",
                    );
                    return None;
                }
                if let Some(reference_span) =
                    self.target_consumption_span(*value, binding.place, true)
                {
                    self.errors.at(
                        "ZRYNA-M3014",
                        span(self.input.sources(), reference_span),
                        "owned aggregate assignment cannot consume its destination while preparing its replacement",
                        "clone the destination or prepare a distinct aggregate value before replacement",
                    );
                    return None;
                }
                let assignment_span = span(self.input.sources(), statement.span);
                if let Some(source) =
                    self.partial_assignment_transfer_source(*value, binding.ty, binding.place)
                {
                    self.lower_partial_assignment_transfer(
                        source,
                        binding.place,
                        binding.ty,
                        assignment_span,
                    )?;
                    return Some(StatementOutcome::Continue);
                }
                if !self.reserve_transition(assignment_span) {
                    return None;
                }
                let Some(prepared) = self.value(*value, binding.ty) else {
                    self.release_transition();
                    return None;
                };
                self.release_transition();
                if !self.emit_effect(
                    assignment_span,
                    raw::InstructionKind::ReplacePlace { place: binding.place, value: prepared },
                ) {
                    return None;
                }
                if self
                    .owners
                    .replace(prepared, binding.place)
                    .map(|delta| self.preparation_facts.apply(delta))
                    .is_none()
                {
                    self.errors.at(
                        "ZRYNA-M3014",
                        assignment_span,
                        "owned aggregate assignment replacement has no distinct prepared owner",
                        "replace from one available independently prepared aggregate value",
                    );
                    return None;
                }
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    span(self.input.sources(), statement.span),
                    "statement is outside straight-line private owned aggregate lowering",
                    "use explicitly typed initialized locals and one final return",
                );
                return None;
            }
        }
        Some(StatementOutcome::Continue)
    }
}
