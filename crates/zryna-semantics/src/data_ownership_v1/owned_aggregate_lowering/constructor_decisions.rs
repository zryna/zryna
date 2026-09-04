use std::collections::BTreeSet;

use zryna_layout as layout;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawDataDeclarationKind, RawFieldInitializerKind};

use super::super::super::layout_graph::semantic_type;
use super::super::{aggregate_graph_is_supported, owned_enum_graph_is_supported};
use super::{ArrayDecision, EnumDecision, ExpressionDecisions, StructDecision, Ty, span};

impl<'f> ExpressionDecisions<'_, 'f, '_> {
    fn ty_for_layout(&self, id: layout::TypeId) -> Option<Ty> {
        self.node_types.iter().flatten().find(|ty| ty.layout == id).copied()
    }

    pub(super) fn struct_decision(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        fields: &[syntax::RawFieldInitializer],
        expected: Option<Ty>,
        at: Span,
    ) -> Option<(Ty, StructDecision)> {
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
        if expected.is_some_and(|expected| actual != expected)
            || !(aggregate_graph_is_supported(actual, self.layouts, &mut BTreeSet::new())
                || super::super::mixed_shape::requires_summary(actual, self.layouts))
        {
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
            let Some((ordinal, _)) = declared
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
            ordered[ordinal] = Some(expression);
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
        Some((
            actual,
            StructDecision {
                children: declared
                    .iter()
                    .zip(ordered.into_iter().flatten())
                    .map(|(declaration, expression)| (declaration.type_syntax, expression))
                    .collect(),
            },
        ))
    }

    pub(super) fn array_decision(
        &mut self,
        type_syntax: u32,
        elements: &'f [u32],
        expected: Ty,
        at: Span,
    ) -> Option<ArrayDecision<'f>> {
        let actual = semantic_type(
            self.file,
            type_syntax,
            self.module,
            self.declarations,
            self.graph,
            self.node_types,
            self.errors,
        )?;
        if actual != expected
            || !(aggregate_graph_is_supported(actual, self.layouts, &mut BTreeSet::new())
                || super::super::mixed_shape::requires_summary(actual, self.layouts))
        {
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
        Some(ArrayDecision { elements, element })
    }

    fn enum_constructor_supported(&self, ty: Ty) -> bool {
        owned_enum_graph_is_supported(ty, self.layouts)
            || super::super::mixed_shape::requires_summary(ty, self.layouts)
    }

    pub(super) fn enum_decision(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        variant_name: &syntax::RawIdentifierSyntax,
        payload: Option<u32>,
        expected: Option<Ty>,
        at: Span,
    ) -> Option<(Ty, EnumDecision)> {
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
        if expected.is_some_and(|expected| actual != expected) {
            self.errors.at(
                "ZRYNA-M3007",
                span(self.input.sources(), name.span),
                "enum constructor has a different exact result type",
                "construct the exact enum required by this context",
            );
            return None;
        }
        if !self.enum_constructor_supported(actual) {
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
        let payload_input = match (variant.payload_type, payload) {
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
                if !aggregate_graph_is_supported(payload_ty, self.layouts, &mut BTreeSet::new())
                    && !super::super::mixed_shape::supported(payload_ty, self.layouts)
                {
                    self.errors.at(
                        "ZRYNA-M3016",
                        span(self.input.sources(), variant_name.span),
                        "enum payload graph is outside the private owned aggregate slice",
                        "use only bool, i32, String, Struct, or fixed-array payloads",
                    );
                    return None;
                }
                Some((expression, payload_ty))
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
        Some((actual, EnumDecision { ordinal, payload_input }))
    }
}
