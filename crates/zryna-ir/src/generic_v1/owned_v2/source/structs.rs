//! Private v4 whole Struct construction and original-Copy field observations.
use super::{Admission, Builder, Closed, Owned, Value};
use crate::generic_v1::{Failure, raw, reject, reserve, source_types::type_id};
use zryna_source::UntrustedSpan;
use zryna_syntax::{
    v4::{RawFieldInitializer, RawFieldInitializerKind, RawIdentifierSyntax},
    v5::{RawExpressionKind, RawTypeArgumentList},
};
mod effects;
mod identity;
mod lookup;

impl Builder<'_, '_> {
    pub(super) fn construct_struct(
        &mut self,
        name: &RawIdentifierSyntax,
        arguments: Option<&RawTypeArgumentList>,
        initializers: &[RawFieldInitializer],
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        if self.type_facts.admission() != Admission::StructV4 {
            return Err(reject("Struct construction requires the distinct private v4 issuer"));
        }
        let mut types = reserve(arguments.map_or(0, |list| list.arguments.len()))?;
        for occurrence in arguments.map_or(&[][..], |list| &list.arguments) {
            let Closed::Stored(key) = self.resolve(*occurrence)? else {
                return Err(self.field_failure(span, "Struct type argument is not stored"));
            };
            types.push(key);
        }
        self.type_facts.description(&self.resolver, &name.text, &types, &mut self.budget)?;
        let description = identity::describe(
            &self.resolver,
            &name.text,
            &types,
            self.symbolic,
            self.type_facts.admission(),
        )?;
        // Complete declaration-name mapping precedes every initializer evaluation/move.
        let mut mapping = reserve(description.fields.len())?;
        mapping.resize(description.fields.len(), None);
        for initializer in initializers {
            let (field, value) = initializer_parts(initializer);
            let Some(ordinal) =
                lookup::ordinal(&description.fields, &field.text, &mut self.budget)?
            else {
                return Err(self.field_failure(field.span, "unknown original Struct field"));
            };
            if mapping[ordinal].replace(value).is_some() {
                return Err(self.field_failure(field.span, "duplicate original Struct initializer"));
            }
        }
        if mapping.iter().any(Option::is_none) {
            return Err(self.field_failure(span, "missing original Struct initializer"));
        }
        let mut fields = reserve(mapping.len())?;
        // Successful fields stay separate live temporaries until the final infallible operation.
        for (ordinal, occurrence) in mapping.into_iter().enumerate() {
            let occurrence = occurrence.ok_or(Failure::InternalFailure)?;
            let value = self.expression(occurrence, depth + 1)?;
            if value.ty != description.fields[ordinal].1 {
                return Err(self.field_failure(
                    self.original.body.expressions[occurrence as usize].span,
                    "Struct field differs from its exact original substituted type",
                ));
            }
            fields.push(value.id);
        }
        let ty = if self.symbolic {
            0
        } else {
            let raw::Type::Stored(id) = type_id(self.program, description.ty.clone())? else {
                return Err(Failure::InternalFailure);
            };
            id
        };
        self.ext(description.ty, span, Owned::StructConstruct { ty, fields })
    }

    pub(super) fn copy_struct_field(
        &mut self,
        base: u32,
        field: &RawIdentifierSyntax,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        if self.type_facts.admission() != Admission::StructV4 {
            return Err(reject("Struct field observation requires the distinct private v4 issuer"));
        }
        let value = if matches!(
            self.original.body.expressions[base as usize].kind,
            RawExpressionKind::Reference { .. }
        ) {
            self.reference_operand(base)?
        } else {
            self.expression(base, depth + 1)?
        };
        let Closed::Stored(key) = &value.ty else {
            return Err(reject("Struct field observation requires a complete stored base"));
        };
        self.type_facts.affine(self.resolver.originals, &value.ty, &mut self.budget)?;
        let description = identity::stored(
            &self.resolver,
            key,
            self.symbolic,
            self.type_facts.admission(),
            &mut self.budget,
        )?;
        let Some(ordinal) = lookup::ordinal(&description.fields, &field.text, &mut self.budget)?
        else {
            return Err(self.field_failure(field.span, "unknown original Struct field"));
        };
        let ty = &description.fields[ordinal].1;
        if self.type_facts.affine(self.resolver.originals, ty, &mut self.budget)? {
            return Err(super::owners::ownership(
                "field observation requires original Copy; opaque or affine fields cannot move partially",
            ));
        }
        self.budget.nominal_key(ty, 1)?;
        self.ext(
            ty.clone(),
            span,
            Owned::CopyStructField {
                value: value.id,
                ordinal: u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
            },
        )
    }

    fn field_failure(&self, span: UntrustedSpan, message: &str) -> Failure {
        let guidance =
            "initialize every exact original field once using its substituted stored type";
        let diagnostic = self.sources.verify_span(span).map_or_else(
            |_| zryna_diagnostics::Diagnostic::error("ZRYNA-M7006", None, message, guidance),
            |span| zryna_diagnostics::Diagnostic::error_at("ZRYNA-M7006", span, message, guidance),
        );
        Failure::Diagnostics(vec![diagnostic])
    }
}

fn initializer_parts(initializer: &RawFieldInitializer) -> (&RawIdentifierSyntax, u32) {
    match &initializer.kind {
        RawFieldInitializerKind::Shorthand { name, value }
        | RawFieldInitializerKind::Explicit { name, value, .. } => (name, *value),
    }
}
