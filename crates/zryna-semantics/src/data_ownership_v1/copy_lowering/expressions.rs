use super::super::function_catalog::{FunctionParameterOrder, FunctionSignature};
use super::super::{
    RawDataDeclarationKind, RawExpressionKind, Span, Ty, TypeCategory, raw, semantic_type, span,
    syntax,
};
use super::FunctionLowerer;

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
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3008",
                        at,
                        format!("integer literal '{spelling}' is outside i32"),
                        "use a decimal i32 literal",
                    );
                    None
                })?;
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
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Add {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Subtraction { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Sub {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Multiplication { lhs, rhs, .. } => {
                self.binary_i32(*lhs, *rhs, at, |lhs, rhs| raw::InstructionKind::I32Mul {
                    lhs,
                    rhs,
                })
            }
            RawExpressionKind::Equal { lhs, rhs, .. } => self.compare(*lhs, *rhs, at, false),
            RawExpressionKind::NotEqual { lhs, rhs, .. } => self.compare(*lhs, *rhs, at, true),
            RawExpressionKind::LessThan { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 0),
            RawExpressionKind::LessEqual { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 1),
            RawExpressionKind::GreaterThan { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 2),
            RawExpressionKind::GreaterEqual { lhs, rhs, .. } => self.rel(*lhs, *rhs, at, 3),
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
                    self.errors.at(
                        "ZRYNA-M3002",
                        span(self.input.sources(), name.span),
                        format!("name '{}' is not declared", name.text),
                        "reference one exact parameter, local, or match payload binding",
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
    fn struct_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        fields: &[syntax::RawFieldInitializer],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = self.decl_ty(&name.text).or_else(|| {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a local aggregate type", name.text),
                "construct an exact declared struct",
            );
            None
        })?;
        let decl =
            self.declarations.iter().find(|d| d.module == self.module && d.name == name.text)?;
        let RawDataDeclarationKind::Struct { fields: declared, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                "struct construction names an enum",
                "use enum variant construction for an enum",
            );
            return None;
        };
        let initializers = planning::struct_initializers(
            self.input.sources(),
            name,
            fields,
            declared,
            self.errors,
        )?;
        let mut values = Vec::with_capacity(declared.len());
        for declared_field in declared {
            let Some((expression, field_span)) =
                initializers.get(&declared_field.name.text).copied()
            else {
                self.errors.at(
                    "ZRYNA-M3005",
                    at,
                    format!("field '{}' is not initialized", declared_field.name.text),
                    "initialize every declared field exactly once",
                );
                return None;
            };
            let value = self.value(expression)?;
            let expected = semantic_type(
                self.file,
                declared_field.type_syntax,
                self.module,
                self.declarations,
                self.graph,
                self.node_types,
                self.errors,
            )?;
            self.require_type(expected, value.0, field_span, "struct field")?;
            values.push(value.1);
        }
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::StructConstruct { fields: values, cleanup: None },
        )?;
        Some((ty, id))
    }
    fn enum_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        variant_name: &syntax::RawIdentifierSyntax,
        payload: Option<u32>,
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = self.decl_ty(&name.text).or_else(|| {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a module-local enum type", name.text),
                "construct one exact declared enum variant",
            );
            None
        })?;
        let decl =
            self.declarations.iter().find(|d| d.module == self.module && d.name == name.text)?;
        let RawDataDeclarationKind::Enum { variants, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                "enum construction names a struct",
                "construct a declared enum variant",
            );
            return None;
        };
        let (ordinal, variant) =
            variants.iter().enumerate().find(|(_, v)| v.name.text == variant_name.text).or_else(
                || {
                    self.errors.at(
                        "ZRYNA-M3005",
                        span(self.input.sources(), variant_name.span),
                        format!("enum '{}' has no variant '{}'", name.text, variant_name.text),
                        "use one exact declared variant",
                    );
                    None
                },
            )?;
        let payload_value = match (variant.payload_type, payload) {
            (None, None) => None,
            (Some(expected), Some(value)) => {
                let payload_span = usize::try_from(value)
                    .ok()
                    .and_then(|index| self.function.body.expressions.get(index))
                    .map_or(at, |expression| span(self.input.sources(), expression.span));
                let expected = semantic_type(
                    self.file,
                    expected,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                let value = self.value(value)?;
                self.require_type(expected, value.0, payload_span, "enum payload")?;
                Some(value.1)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3005",
                    at,
                    "enum payload presence does not match the declared variant",
                    "supply exactly one payload only for a payload variant",
                );
                return None;
            }
        };
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::EnumConstruct {
                variant: u32::try_from(ordinal).ok()?,
                payload: payload_value,
                cleanup: None,
            },
        )?;
        Some((ty, id))
    }
    fn array_value(
        &mut self,
        type_syntax: u32,
        elements: &[u32],
        at: Span,
    ) -> Option<(Ty, raw::ValueId)> {
        let ty = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        let record = self.layouts.type_by_id(ty.layout)?;
        let length = record.array_length()?;
        if u64::try_from(elements.len()).ok()? != length {
            self.errors.at(
                "ZRYNA-M3005",
                at,
                format!(
                    "fixed-array constructor has {} elements but its type requires {length}",
                    elements.len()
                ),
                "provide exactly the fixed-array length",
            );
            return None;
        }
        let element = record.referenced_type()?;
        let element_ty =
            self.node_types.iter().flatten().find(|ty| ty.layout == element).copied()?;
        let mut values = Vec::with_capacity(elements.len());
        for expression in elements {
            let element_span = usize::try_from(*expression)
                .ok()
                .and_then(|index| self.function.body.expressions.get(index))
                .map_or(at, |value| span(self.input.sources(), value.span));
            let value = self.value(*expression)?;
            self.require_type(element_ty, value.0, element_span, "fixed-array element")?;
            values.push(value.1);
        }
        let id = self.emit(
            Some(ty),
            at,
            raw::InstructionKind::FixedArrayConstruct { elements: values, cleanup: None },
        )?;
        Some((ty, id))
    }
    fn unary_i32(&mut self, operand: u32, at: Span) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let value = self.value(operand)?;
        self.require_type(expected, value.0, at, "negation operand")?;
        let id =
            self.emit(Some(expected), at, raw::InstructionKind::I32Neg { operand: value.1 })?;
        Some((expected, id))
    }
    fn binary_i32(
        &mut self,
        lhs: u32,
        rhs: u32,
        at: Span,
        make: impl FnOnce(raw::ValueId, raw::ValueId) -> raw::InstructionKind,
    ) -> Option<(Ty, raw::ValueId)> {
        let expected = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(expected, lhs.0, at, "left operand")?;
        self.require_type(expected, rhs.0, at, "right operand")?;
        let id = self.emit(Some(expected), at, make(lhs.1, rhs.1))?;
        Some((expected, id))
    }
    fn compare(&mut self, lhs: u32, rhs: u32, at: Span, not: bool) -> Option<(Ty, raw::ValueId)> {
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(lhs.0, rhs.0, at, "comparison")?;
        if !matches!(lhs.0.category, TypeCategory::Bool | TypeCategory::I32) {
            self.errors.at(
                "ZRYNA-M3008",
                at,
                "equality is scalar-only in aggregate M3",
                "compare bool or i32 projections rather than whole aggregates",
            );
            return None;
        }
        let result = self.primitive(TypeCategory::Bool)?;
        let kind = if not {
            raw::InstructionKind::Ne { lhs: lhs.1, rhs: rhs.1 }
        } else {
            raw::InstructionKind::Eq { lhs: lhs.1, rhs: rhs.1 }
        };
        let id = self.emit(Some(result), at, kind)?;
        Some((result, id))
    }
    fn rel(&mut self, lhs: u32, rhs: u32, at: Span, op: u8) -> Option<(Ty, raw::ValueId)> {
        let i32_ty = self.primitive(TypeCategory::I32)?;
        let lhs = self.value(lhs)?;
        let rhs = self.value(rhs)?;
        self.require_type(i32_ty, lhs.0, at, "relational operand")?;
        self.require_type(i32_ty, rhs.0, at, "relational operand")?;
        let kind = match op {
            0 => raw::InstructionKind::I32LtS { lhs: lhs.1, rhs: rhs.1 },
            1 => raw::InstructionKind::I32LeS { lhs: lhs.1, rhs: rhs.1 },
            2 => raw::InstructionKind::I32GtS { lhs: lhs.1, rhs: rhs.1 },
            _ => raw::InstructionKind::I32GeS { lhs: lhs.1, rhs: rhs.1 },
        };
        let result = self.primitive(TypeCategory::Bool)?;
        let id = self.emit(Some(result), at, kind)?;
        Some((result, id))
    }
}
