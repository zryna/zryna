use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind, RawFieldInitializerKind};

use super::borrow_call_resources::{
    BorrowCallPreflightError, checked_add_resources, checked_call_delta, checked_merge_estimates,
    one_value_transition,
};
use super::borrow_forwarding::plan_forwarded_borrow_arguments;
use super::function_catalog::{FunctionParameterOrder, FunctionResolution, FunctionSignature};
use super::layout_graph::semantic_type;
use super::type_model::{RootBorrowBudgetLimit, RootBorrowResources, Ty};
use super::{FunctionLowerer, span};

struct BorrowCallExpressionPreflight {
    resources: RootBorrowResources,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    next_place: usize,
}

impl BorrowCallExpressionPreflight {
    fn new(lowerer: &FunctionLowerer<'_, '_, '_>) -> Self {
        Self {
            resources: RootBorrowResources::default(),
            projections: lowerer.projections.clone(),
            next_place: lowerer.places.len(),
        }
    }

    fn merge(&mut self, additional: RootBorrowResources) -> Option<()> {
        self.resources = checked_merge_estimates(self.resources, additional)?;
        Some(())
    }

    fn place(&mut self) -> Option<raw::PlaceId> {
        let id = raw::PlaceId(u32::try_from(self.next_place).ok()?);
        self.next_place = self.next_place.checked_add(1)?;
        self.resources.places = self.resources.places.checked_add(1)?;
        Some(id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FunctionLowererMutationSnapshot {
    values: u32,
    places: usize,
    instructions: usize,
    cleanup_plans: usize,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
}

#[cfg(test)]
impl FunctionLowererMutationSnapshot {
    pub(super) fn shape(&self) -> (u32, usize, usize, usize, usize) {
        (self.values, self.places, self.instructions, self.cleanup_plans, self.projections.len())
    }
}

impl FunctionLowerer<'_, '_, '_> {
    pub(super) fn resolve_copy_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        at: Span,
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
                "ZRYNA-M3008",
                span(self.input.sources(), callee.span),
                "this checkpoint admits calls only to private same-module functions",
                "keep the called function internal",
            );
            return None;
        }
        if !signature.result.is_copy() || signature.parameters.iter().any(|ty| !ty.is_copy()) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), callee.span),
                "owned direct-call transfer is outside the current Copy-call checkpoint",
                "call only exact bool, i32, or Copy aggregate signatures",
            );
            return None;
        }
        if arguments.len() != signature.parameter_order.len() {
            self.errors.at(
                "ZRYNA-M3008",
                at,
                format!(
                    "call to '{}' has {} arguments but its signature requires {}",
                    signature.name,
                    arguments.len(),
                    signature.parameter_order.len()
                ),
                "pass one argument for every exact declared parameter",
            );
            return None;
        }
        Some(signature)
    }

    #[allow(clippy::too_many_lines)]
    fn estimate_call_value(
        &mut self,
        id: u32,
        estimate: &mut BorrowCallExpressionPreflight,
    ) -> Option<Ty> {
        let expression = usize::try_from(id)
            .ok()
            .and_then(|index| self.function.body.expressions.get(index))?
            .clone();
        let at = span(self.input.sources(), expression.span);
        let ty = match expression.kind {
            RawExpressionKind::Reference { name } => self
                .borrow_bindings
                .get(&name.text)
                .map(|binding| binding.ty)
                .or_else(|| self.bindings.get(&name.text).map(|binding| binding.ty))
                .or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("name '{}' is not declared", name.text),
                        "reference one exact parameter, local, or match payload binding",
                    );
                    None
                })?,
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                self.estimate_call_place(id, estimate)?.0
            }
            RawExpressionKind::BoolLiteral { .. } => self.primitive(TypeCategory::Bool)?,
            RawExpressionKind::I32Literal { ref spelling } => {
                if spelling.parse::<i32>().is_err() {
                    self.errors.at(
                        "ZRYNA-M3008",
                        at,
                        format!("integer literal '{spelling}' is outside i32"),
                        "use a decimal i32 literal",
                    );
                    return None;
                }
                self.primitive(TypeCategory::I32)?
            }
            RawExpressionKind::StructConstruction { type_name, fields, .. } => {
                for field in fields {
                    let value = match field.kind {
                        RawFieldInitializerKind::Shorthand { value, .. }
                        | RawFieldInitializerKind::Explicit { value, .. } => value,
                    };
                    self.estimate_call_value(value, estimate)?;
                }
                self.decl_ty(&type_name.text).or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3005",
                        span(self.input.sources(), type_name.span),
                        format!("'{}' is not a local aggregate type", type_name.text),
                        "construct an exact declared struct",
                    );
                    None
                })?
            }
            RawExpressionKind::EnumConstruction { type_name, payload, .. } => {
                if let Some(payload) = payload {
                    self.estimate_call_value(payload, estimate)?;
                }
                self.decl_ty(&type_name.text).or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3005",
                        span(self.input.sources(), type_name.span),
                        format!("'{}' is not a module-local enum type", type_name.text),
                        "construct one exact declared enum variant",
                    );
                    None
                })?
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. } => {
                for element in elements {
                    self.estimate_call_value(element, estimate)?;
                }
                semantic_type(
                    self.file,
                    type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?
            }
            RawExpressionKind::Call { callee, arguments, .. } => {
                let signature = self.resolve_copy_call(&callee, &arguments, at)?;
                plan_forwarded_borrow_arguments(
                    self.input.sources(),
                    self.function,
                    &signature,
                    &arguments,
                    &self.borrow_bindings,
                    self.errors,
                )?;
                for (argument, order) in arguments.iter().zip(&signature.parameter_order) {
                    let FunctionParameterOrder::Value(index) = *order else {
                        continue;
                    };
                    let expected = *signature.parameters.get(usize::try_from(index).ok()?)?;
                    let actual = self.estimate_call_value(*argument, estimate)?;
                    let argument_at = span(
                        self.input.sources(),
                        self.function.body.expressions[*argument as usize].span,
                    );
                    self.require_type(expected, actual, argument_at, "call argument")?;
                }
                estimate.resources =
                    checked_call_delta(estimate.resources, false).or_else(|| {
                        self.errors.at(
                        "ZRYNA-M3201",
                        at,
                        "nested borrow-call preparation overflows its checked resource estimate",
                        "reduce nested Copy call arguments",
                    );
                        None
                    })?;
                return Some(signature.result);
            }
            RawExpressionKind::Negation { operand, .. } => {
                self.estimate_call_value(operand, estimate)?;
                self.primitive(TypeCategory::I32)?
            }
            RawExpressionKind::Addition { lhs, rhs, .. }
            | RawExpressionKind::Subtraction { lhs, rhs, .. }
            | RawExpressionKind::Multiplication { lhs, rhs, .. } => {
                self.estimate_call_value(lhs, estimate)?;
                self.estimate_call_value(rhs, estimate)?;
                self.primitive(TypeCategory::I32)?
            }
            RawExpressionKind::Equal { lhs, rhs, .. }
            | RawExpressionKind::NotEqual { lhs, rhs, .. }
            | RawExpressionKind::LessThan { lhs, rhs, .. }
            | RawExpressionKind::LessEqual { lhs, rhs, .. }
            | RawExpressionKind::GreaterThan { lhs, rhs, .. }
            | RawExpressionKind::GreaterEqual { lhs, rhs, .. } => {
                self.estimate_call_value(lhs, estimate)?;
                self.estimate_call_value(rhs, estimate)?;
                self.primitive(TypeCategory::Bool)?
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3008",
                    at,
                    "expression is outside deterministic aggregate M3",
                    "use Copy construction, projection, or scalar operations",
                );
                return None;
            }
        };
        estimate.merge(one_value_transition()).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "borrow-call argument preparation overflows its checked resource estimate",
                "reduce nested Copy call arguments",
            );
            None
        })?;
        Some(ty)
    }

    fn estimate_call_place(
        &mut self,
        id: u32,
        estimate: &mut BorrowCallExpressionPreflight,
    ) -> Option<(Ty, raw::PlaceId)> {
        let expression = usize::try_from(id)
            .ok()
            .and_then(|index| self.function.body.expressions.get(index))?
            .clone();
        match expression.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).map(|binding| (binding.ty, binding.place)).or_else(
                    || {
                        self.errors.at(
                            "ZRYNA-M3002",
                            span(self.input.sources(), name.span),
                            format!("name '{}' is not declared", name.text),
                            "reference one exact parameter, local, or match payload binding",
                        );
                        None
                    },
                )
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let (base_ty, base_place) = self.estimate_call_place(base, estimate)?;
                let (ordinal, ty) =
                    self.field(base_ty, &field.text, span(self.input.sources(), field.span))?;
                let key = (base_place.0, 0, ordinal);
                let place = if let Some(place) = estimate.projections.get(&key).copied() {
                    place
                } else {
                    let place = estimate.place()?;
                    estimate.projections.insert(key, place);
                    place
                };
                Some((ty, place))
            }
            RawExpressionKind::Index { base, index, .. } => {
                let (base_ty, base_place) = self.estimate_call_place(base, estimate)?;
                let (ordinal, ty) = self.constant_index(base_ty, index)?;
                let key = (base_place.0, 1, ordinal);
                let place = if let Some(place) = estimate.projections.get(&key).copied() {
                    place
                } else {
                    let place = estimate.place()?;
                    estimate.projections.insert(key, place);
                    place
                };
                Some((ty, place))
            }
            RawExpressionKind::StructConstruction { .. }
            | RawExpressionKind::EnumConstruction { .. }
            | RawExpressionKind::FixedArrayConstruction { .. } => {
                let ty = self.estimate_call_value(id, estimate)?;
                Some((ty, estimate.place()?))
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3006",
                    span(self.input.sources(), expression.span),
                    "projection base is not an addressable aggregate place",
                    "project from a parameter, local, aggregate constructor, field, or fixed-array element",
                );
                None
            }
        }
    }

    pub(super) fn preflight_copy_borrow_call(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[u32],
        at: Span,
    ) -> Option<Vec<Option<raw::BorrowId>>> {
        let borrows = plan_forwarded_borrow_arguments(
            self.input.sources(),
            self.function,
            signature,
            arguments,
            &self.borrow_bindings,
            self.errors,
        )?;
        let mut estimate = BorrowCallExpressionPreflight::new(self);
        for (argument, order) in arguments.iter().zip(&signature.parameter_order) {
            let FunctionParameterOrder::Value(index) = *order else {
                continue;
            };
            let expected = *signature.parameters.get(usize::try_from(index).ok()?)?;
            let actual = self.estimate_call_value(*argument, &mut estimate)?;
            let argument_at =
                span(self.input.sources(), self.function.body.expressions[*argument as usize].span);
            self.require_type(expected, actual, argument_at, "call argument")?;
        }
        let additional = checked_call_delta(estimate.resources, false).or_else(|| {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "borrow-call preparation overflows its checked resource estimate",
                "reduce nested Copy call arguments",
            );
            None
        })?;
        let current = RootBorrowResources {
            values: usize::try_from(self.values).unwrap_or(usize::MAX),
            places: self.places.len(),
            transitions: self.instructions.len(),
            blocks: 1,
            edges: 0,
            active_peak: self.borrow_bindings.len(),
            cleanup_plans: self.cleanup_plans.len(),
        };
        if let Err(error) = checked_add_resources(current, additional) {
            let (message, guidance) = match error {
                BorrowCallPreflightError::Overflow => (
                    "borrow-call resource reservation overflows checked arithmetic".to_owned(),
                    "reduce nested Copy call arguments",
                ),
                BorrowCallPreflightError::Limit(limit) => {
                    let label = match limit {
                        RootBorrowBudgetLimit::Values => "derived values",
                        RootBorrowBudgetLimit::Places => "derived places",
                        RootBorrowBudgetLimit::Transitions => "derived ownership transitions",
                        RootBorrowBudgetLimit::Blocks => "derived blocks",
                        RootBorrowBudgetLimit::Edges => "derived control-flow edges",
                        RootBorrowBudgetLimit::ActiveBorrows => "simultaneously active borrows",
                        RootBorrowBudgetLimit::CleanupPlans => "derived cleanup plans",
                    };
                    (
                        format!(
                            "borrow-call preparation exceeds the per-function limit for {label}"
                        ),
                        "reduce nested Copy call arguments or borrow parameters",
                    )
                }
            };
            self.errors.at("ZRYNA-M3201", at, message, guidance);
            return None;
        }
        Some(borrows)
    }

    pub(super) fn mutation_snapshot(&self) -> FunctionLowererMutationSnapshot {
        FunctionLowererMutationSnapshot {
            values: self.values,
            places: self.places.len(),
            instructions: self.instructions.len(),
            cleanup_plans: self.cleanup_plans.len(),
            projections: self.projections.clone(),
        }
    }

    pub(super) fn restore_mutation_snapshot(&mut self, snapshot: FunctionLowererMutationSnapshot) {
        self.values = snapshot.values;
        self.places.truncate(snapshot.places);
        self.instructions.truncate(snapshot.instructions);
        self.cleanup_plans.truncate(snapshot.cleanup_plans);
        self.projections = snapshot.projections;
    }
}
