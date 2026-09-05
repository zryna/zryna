use super::super::function_catalog::{FunctionParameterOrder, FunctionSignature};
use super::super::scalar_operations::{self, ScalarOperation};
use super::super::{RawExpressionKind, Span, Ty, TypeCategory, raw, syntax};
use super::FunctionLowerer;
use crate::data_ownership_v1::diagnostics::span;

mod constructors;
mod planning;

impl FunctionLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn value(&mut self, id: u32) -> Option<(Ty, raw::ValueId)> {
        let expr = usize::try_from(id).ok().and_then(|i| self.function.body.expressions.get(i))?;
        let at = span(self.input.sources(), expr.span);
        match &expr.kind {
            RawExpressionKind::Reference { name } => {
                if let Some(binding) = self.borrow_bindings.get(&name.text).copied() {
                    let value = self.emit(
                        Some(binding.ty),
                        at,
                        raw::InstructionKind::BorrowRead { borrow: binding.borrow },
                    )?;
                    return Some((binding.ty, value));
                }
                let (ty, place, _) = self.place(id)?;
                let value =
                    self.emit(Some(ty), at, raw::InstructionKind::CopyFromPlace { place })?;
                Some((ty, value))
            }
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                let (ty, place, _) = self.place(id)?;
                let value =
                    self.emit(Some(ty), at, raw::InstructionKind::CopyFromPlace { place })?;
                Some((ty, value))
            }
            RawExpressionKind::BoolLiteral { value } => {
                let ty = self.primitive(TypeCategory::Bool)?;
                let id = self.emit(Some(ty), at, raw::InstructionKind::BoolLiteral(*value))?;
                Some((ty, id))
            }
            RawExpressionKind::I32Literal { spelling } => {
                let value = scalar_operations::integer(spelling, at, self.errors)?;
                let ty = self.primitive(TypeCategory::I32)?;
                let id = self.emit(Some(ty), at, raw::InstructionKind::I32Literal(value))?;
                Some((ty, id))
            }
            RawExpressionKind::StructConstruction { type_name, fields, .. } => {
                self.struct_value(type_name, fields, at)
            }
            RawExpressionKind::EnumConstruction { type_name, variant, payload, .. } => {
                self.enum_value(type_name, variant, *payload, at)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. } => {
                self.array_value(*type_syntax, elements, at)
            }
            RawExpressionKind::Call { callee, arguments, .. } => {
                self.direct_call(callee, arguments, at)
            }
            RawExpressionKind::Negation { operand, .. } => self.unary_i32(*operand, at),
            RawExpressionKind::Addition { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, ScalarOperation::Add)
            }
            RawExpressionKind::Subtraction { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, ScalarOperation::Sub)
            }
            RawExpressionKind::Multiplication { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, ScalarOperation::Mul)
            }
            RawExpressionKind::Equal { lhs, rhs, .. } => {
                self.compare(*lhs, *rhs, at, ScalarOperation::Eq)
            }
            RawExpressionKind::NotEqual { lhs, rhs, .. } => {
                self.compare(*lhs, *rhs, at, ScalarOperation::Ne)
            }
            RawExpressionKind::LessThan { lhs, rhs, .. } => {
                self.rel(*lhs, *rhs, at, ScalarOperation::Lt)
            }
            RawExpressionKind::LessEqual { lhs, rhs, .. } => {
                self.rel(*lhs, *rhs, at, ScalarOperation::Le)
            }
            RawExpressionKind::GreaterThan { lhs, rhs, .. } => {
                self.rel(*lhs, *rhs, at, ScalarOperation::Gt)
            }
            RawExpressionKind::GreaterEqual { lhs, rhs, .. } => {
                self.rel(*lhs, *rhs, at, ScalarOperation::Ge)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3008",
                    at,
                    "expression is outside deterministic aggregate M3",
                    "use Copy construction, projection, or scalar operations",
                );
                None
            }
        }
    }
    pub(in crate::data_ownership_v1) fn lower_direct_call_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[u32],
        borrows: Vec<Option<raw::BorrowId>>,
    ) -> Option<Vec<raw::CallArgument>> {
        let mut values = vec![None; signature.parameters.len()];
        for (argument, order) in arguments.iter().zip(&signature.parameter_order) {
            let argument_span =
                span(self.input.sources(), self.function.body.expressions[*argument as usize].span);
            match *order {
                FunctionParameterOrder::Value(index) => {
                    let expected = *signature.parameters.get(usize::try_from(index).ok()?)?;
                    let (actual, value) = self.value(*argument)?;
                    self.require_type(expected, actual, argument_span, "call argument")?;
                    *values.get_mut(usize::try_from(index).ok()?)? = Some(value);
                }
                FunctionParameterOrder::Borrow(_) => {}
            }
        }
        let mut lowered = Vec::with_capacity(arguments.len());
        for value in values {
            lowered.push(raw::CallArgument::Value(value?));
        }
        for borrow in borrows {
            lowered.push(raw::CallArgument::Borrow(borrow?));
        }
        Some(lowered)
    }

    pub(in crate::data_ownership_v1) fn direct_call(
        &mut self,
        callee: &syntax::RawIdentifierSyntax,
        arguments: &[u32],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let signature = self.resolve_copy_call(callee, arguments, at)?;
        let borrows = self.preflight_copy_borrow_call(&signature, arguments, at)?;
        let snapshot = self.mutation_snapshot();
        let expected_after_rollback = snapshot.clone();
        let Some(lowered) = self.lower_direct_call_arguments(&signature, arguments, borrows) else {
            self.restore_mutation_snapshot(snapshot);
            debug_assert_eq!(self.mutation_snapshot(), expected_after_rollback);
            return None;
        };
        let cleanup = raw::CleanupPlanId(u32::try_from(self.cleanup_plans.len()).ok()?);
        self.cleanup_plans.push(raw::CleanupPlan { id: cleanup, span: at, actions: Vec::new() });
        let value = self.emit(
            Some(signature.result),
            at,
            raw::InstructionKind::DirectCall { callee: signature.id, arguments: lowered, cleanup },
        )?;
        Some((signature.result, value))
    }
    pub(in crate::data_ownership_v1) fn place(
        &mut self,
        id: u32,
    ) -> Option<(Ty, raw::PlaceId, bool)> {
        let expr = usize::try_from(id).ok().and_then(|i| self.function.body.expressions.get(i))?;
        match &expr.kind {
            RawExpressionKind::Reference { name } => {
                self.bindings.get(&name.text).map(|b| (b.ty, b.place, b.mutable)).or_else(|| {
                    scalar_operations::missing_reference(
                        &name.text,
                        span(self.input.sources(), name.span),
                        self.errors,
                    );
                    None
                })
            }
            RawExpressionKind::FieldAccess { base, field, .. } => {
                let (base_ty, base_place, mutable) = self.place(*base)?;
                let (ordinal, ty) =
                    self.field(base_ty, &field.text, span(self.input.sources(), field.span))?;
                let key = (base_place.0, 0, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place = self.push_place(
                        ty,
                        span(self.input.sources(), expr.span),
                        raw::PlaceKind::StructField { base: base_place, ordinal },
                    );
                    self.projections.insert(key, place);
                    place
                };
                Some((ty, place, mutable))
            }
            RawExpressionKind::Index { base, index, .. } => {
                let (base_ty, base_place, mutable) = self.place(*base)?;
                let (ordinal, ty) = self.constant_index(base_ty, *index)?;
                let key = (base_place.0, 1, ordinal);
                let place = if let Some(place) = self.projections.get(&key).copied() {
                    place
                } else {
                    let place = self.push_place(
                        ty,
                        span(self.input.sources(), expr.span),
                        raw::PlaceKind::FixedArrayConstant { base: base_place, index: ordinal },
                    );
                    self.projections.insert(key, place);
                    place
                };
                Some((ty, place, mutable))
            }
            RawExpressionKind::StructConstruction { .. }
            | RawExpressionKind::EnumConstruction { .. }
            | RawExpressionKind::FixedArrayConstruction { .. } => {
                let (ty, value) = self.value(id)?;
                let place = self.push_place(
                    ty,
                    span(self.input.sources(), expr.span),
                    raw::PlaceKind::Temporary(value),
                );
                Some((ty, place, false))
            }
            _ => {
                self.errors.at("ZRYNA-M3006", span(self.input.sources(), expr.span), "projection base is not an addressable aggregate place", "project from a parameter, local, aggregate constructor, field, or fixed-array element");
                None
            }
        }
    }
    fn unary_i32(&mut self, operand: u32, at: Span) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let value = self.value(operand)?;
        ScalarOperation::Neg.validate(Some(expected), value.0, None, at, self.errors)?;
        let id = self.emit(Some(expected), at, ScalarOperation::Neg.instruction(value.1, None))?;
        Some((expected, id))
    }
    fn binary_i32(
        &mut self,
        lhs: u32,
        rhs: u32,
        at: Span,
        operation: ScalarOperation,
    ) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        operation.validate(Some(expected), lhs.0, Some(rhs.0), at, self.errors)?;
        let id = self.emit(Some(expected), at, operation.instruction(lhs.1, Some(rhs.1)))?;
        Some((expected, id))
    }
    fn compare(
        &mut self,
        lhs: u32,
        rhs: u32,
        at: Span,
        operation: ScalarOperation,
    ) -> Option<(Ty, raw::ValueId)> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        operation.validate(None, lhs.0, Some(rhs.0), at, self.errors)?;
        let result = self.primitive(TypeCategory::Bool)?;
        let id = self.emit(Some(result), at, operation.instruction(lhs.1, Some(rhs.1)))?;
        Some((result, id))
    }
    fn rel(
        &mut self,
        lhs: u32,
        rhs: u32,
        at: Span,
        operation: ScalarOperation,
    ) -> Option<(Ty, raw::ValueId)> {
        let i32_ty = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        operation.validate(Some(i32_ty), lhs.0, Some(rhs.0), at, self.errors)?;
        let result = self.primitive(TypeCategory::Bool)?;
        let id = self.emit(Some(result), at, operation.instruction(lhs.1, Some(rhs.1)))?;
        Some((result, id))
    }
}
