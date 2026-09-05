use super::super::super::{RawDataDeclarationKind, Span, Ty, raw, semantic_type, syntax};
use super::{FunctionLowerer, planning};
use crate::data_ownership_v1::diagnostics::span;

impl FunctionLowerer<'_, '_, '_> {
    pub(super) fn struct_value(
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
    pub(super) fn enum_value(
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
    pub(super) fn array_value(
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
}
