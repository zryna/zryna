use std::collections::BTreeSet;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory};
use zryna_source::Span;
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawFieldInitializerKind,
};

use super::super::layout_graph::semantic_type;
use super::super::span;
use super::super::type_model::Ty;
use super::{
    PrivateOwnedAggregateLowerer, aggregate_graph_is_supported, owned_enum_graph_is_supported,
};

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn ty_for_layout(&self, id: layout::TypeId) -> Option<Ty> {
        self.node_types.iter().flatten().find(|ty| ty.layout == id).copied()
    }

    pub(super) fn value(&mut self, id: u32, expected: Ty) -> Option<raw::ValueId> {
        let expression = self.expression(id)?.clone();
        let at = span(self.input.sources(), expression.span);
        match expression.kind {
            RawExpressionKind::BoolLiteral { value } if expected.category == TypeCategory::Bool => {
                self.emit(expected, at, raw::InstructionKind::BoolLiteral(value))
            }
            RawExpressionKind::I32Literal { spelling }
                if expected.category == TypeCategory::I32 =>
            {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    self.errors.at(
                        "ZRYNA-M3016",
                        at,
                        "aggregate leaf integer is outside i32",
                        "use one in-range i32 leaf",
                    );
                    None
                })?;
                self.emit(expected, at, raw::InstructionKind::I32Literal(value))
            }
            RawExpressionKind::StringLiteral { spelling }
                if expected.category == TypeCategory::String =>
            {
                let bytes = spelling.get(1..spelling.len().checked_sub(1)?)?.as_bytes().to_vec();
                let cleanup = self.push_cleanup(at, None)?;
                self.emit(expected, at, raw::InstructionKind::StringFromUtf8 { bytes, cleanup })
            }
            RawExpressionKind::Reference { name } => self.reference_value(&name, expected, at),
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. } => {
                self.projected_value(id, expected, None)
            }
            RawExpressionKind::Clone { value, .. } if expected.category == TypeCategory::String => {
                self.clone_projected_string(value, expected, at)
            }
            RawExpressionKind::Clone { value, .. } => self.clone_aggregate(value, expected, at),
            RawExpressionKind::StructConstruction { type_name, fields, .. }
                if expected.category == TypeCategory::Struct =>
            {
                self.struct_value(&type_name, &fields, expected, at)
            }
            RawExpressionKind::FixedArrayConstruction { type_syntax, elements, .. }
                if expected.category == TypeCategory::FixedArray =>
            {
                self.array_value(type_syntax, &elements, expected, at)
            }
            RawExpressionKind::EnumConstruction { type_name, variant, payload, .. }
                if expected.category == TypeCategory::Enum =>
            {
                self.enum_value(&type_name, &variant, payload, expected, at)
            }
            _ => {
                self.errors.at(
                    "ZRYNA-M3016",
                    at,
                    "expression is outside private owned Struct/Enum/FixedArray lowering",
                    "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
                );
                None
            }
        }
    }

    fn struct_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        fields: &[syntax::RawFieldInitializer],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let decl = self
            .declarations
            .iter()
            .find(|decl| decl.module == self.module && decl.name == name.text)
            .cloned();
        let Some(decl) = decl else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                format!("'{}' is not an exact module-local owned struct", name.text),
                "construct one exact supported struct type",
            );
            return None;
        };
        let actual = self.node_types.get(decl.node.0 as usize).and_then(|ty| *ty)?;
        if actual != expected || !self.supported(actual) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "struct constructor type or ownership graph is outside the exact supported slice",
                "use an acyclic struct containing only bool, i32, String, or supported fixed arrays",
            );
            return None;
        }
        let RawDataDeclarationKind::Struct { fields: declared, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "owned struct construction names an enum",
                "construct one exact supported struct",
            );
            return None;
        };
        let declared = declared.clone();
        let mut ordered = vec![None; declared.len()];
        for field in fields {
            let (field_name, expression) = match &field.kind {
                RawFieldInitializerKind::Shorthand { name, value }
                | RawFieldInitializerKind::Explicit { name, value, .. } => (&name.text, *value),
            };
            let Some((ordinal, declaration)) = declared
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.name.text == *field_name)
            else {
                self.errors.at(
                    "ZRYNA-M3016",
                    span(self.input.sources(), field.span),
                    format!("struct '{}' has no field '{field_name}'", name.text),
                    "initialize every exact declared field once",
                );
                return None;
            };
            if ordered[ordinal].is_some() {
                self.errors.at(
                    "ZRYNA-M3016",
                    span(self.input.sources(), field.span),
                    format!("field '{field_name}' is initialized more than once"),
                    "initialize every exact declared field once",
                );
                return None;
            }
            let field_ty = semantic_type(
                self.file,
                declaration.type_syntax,
                self.module,
                self.declarations,
                self.graph,
                self.node_types,
                self.errors,
            )?;
            ordered[ordinal] = Some(self.value(expression, field_ty)?);
        }
        if let Some((ordinal, _)) = ordered.iter().enumerate().find(|(_, value)| value.is_none()) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!("field '{}' is not initialized", declared[ordinal].name.text),
                "initialize every exact declared field once",
            );
            return None;
        }
        let values = ordered.into_iter().flatten().collect::<Vec<_>>();
        self.reserve_operands(values.len(), at)?;
        let consumed = self.prevalidate_constructor_operands(&values, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::StructConstruct { fields: values.clone(), cleanup: None },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }

    fn array_value(
        &mut self,
        type_syntax: u32,
        elements: &[u32],
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let actual = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if actual != expected || !self.supported(actual) {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "fixed-array constructor type or ownership graph differs from context",
                "construct the exact supported fixed-array type",
            );
            return None;
        }
        let record = self.layouts.type_by_id(actual.layout)?;
        let length = usize::try_from(record.array_length()?).ok()?;
        if elements.len() != length {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                format!(
                    "fixed-array constructor has {} elements but requires {length}",
                    elements.len()
                ),
                "provide exactly the fixed-array length",
            );
            return None;
        }
        let element = self.ty_for_layout(record.referenced_type()?)?;
        let mut values = Vec::with_capacity(elements.len());
        for expression in elements {
            values.push(self.value(*expression, element)?);
        }
        self.reserve_operands(values.len(), at)?;
        let consumed = self.prevalidate_constructor_operands(&values, at)?;
        let result = self.emit(
            expected,
            at,
            raw::InstructionKind::FixedArrayConstruct { elements: values.clone(), cleanup: None },
        )?;
        self.commit_constructor_operands(&consumed);
        Some(result)
    }

    fn enum_value(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        variant_name: &syntax::RawIdentifierSyntax,
        payload: Option<u32>,
        expected: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        let Some(decl) = self
            .declarations
            .iter()
            .find(|decl| decl.module == self.module && decl.name == name.text)
            .cloned()
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                format!("'{}' is not a module-local enum type", name.text),
                "construct one exact declared enum variant",
            );
            return None;
        };
        let Some(actual) = self.node_types.get(decl.node.0 as usize).and_then(|ty| *ty) else {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "enum constructor type has no authenticated semantic identity",
                "construct one exact supported enum type",
            );
            return None;
        };
        if actual != expected {
            self.errors.at(
                "ZRYNA-M3007",
                span(self.input.sources(), name.span),
                "enum constructor has a different exact result type",
                "construct the exact enum required by this context",
            );
            return None;
        }
        if !owned_enum_graph_is_supported(actual, self.layouts) {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "enum constructor type or payload graph is outside the exact supported slice",
                "use an acyclic enum with payloadless variants or bool, i32, String, Struct, or fixed-array payloads",
            );
            return None;
        }
        let RawDataDeclarationKind::Enum { variants, .. } =
            &self.file.data_declarations()[decl.declaration].kind
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), name.span),
                "owned enum construction names a struct",
                "construct a declared enum variant",
            );
            return None;
        };
        let Some((ordinal, variant)) =
            variants.iter().enumerate().find(|(_, variant)| variant.name.text == variant_name.text)
        else {
            self.errors.at(
                "ZRYNA-M3005",
                span(self.input.sources(), variant_name.span),
                format!("enum '{}' has no variant '{}'", name.text, variant_name.text),
                "use one exact declared variant",
            );
            return None;
        };
        let payload_value = match (variant.payload_type, payload) {
            (None, None) => None,
            (Some(type_syntax), Some(expression)) => {
                let payload_ty = semantic_type(
                    self.file,
                    type_syntax,
                    self.module,
                    self.declarations,
                    self.graph,
                    self.node_types,
                    self.errors,
                )?;
                if !aggregate_graph_is_supported(payload_ty, self.layouts, &mut BTreeSet::new()) {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), variant_name.span),
                        "enum payload graph is outside the private owned aggregate slice",
                        "use only bool, i32, String, Struct, or fixed-array payloads",
                    );
                    return None;
                }
                Some(self.value(expression, payload_ty)?)
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
        self.commit_enum(expected, at, ordinal, payload_value)
    }
}
