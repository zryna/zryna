//! Original nongeneric struct initializer validation and diagnostic order.

use super::{
    BodyTypeFailure, Checker, DeclarationIdentity, RawDataDeclarationKind, RawFieldInitializer,
    RawFieldInitializerKind, RawIdentifierSyntax, constraints, legacy_error, resources, signatures,
    substitution,
};

fn field_value(initializer: &RawFieldInitializer) -> (&RawIdentifierSyntax, u32) {
    match &initializer.kind {
        RawFieldInitializerKind::Shorthand { name, value }
        | RawFieldInitializerKind::Explicit { name, value, .. } => (name, *value),
    }
}

pub(super) fn structure(
    checker: &mut Checker<'_, '_>,
    owner: DeclarationIdentity,
    target: DeclarationIdentity,
    name: &RawIdentifierSyntax,
    initializers: &[RawFieldInitializer],
    at: zryna_source::UntrustedSpan,
) -> Result<bool, BodyTypeFailure> {
    let RawDataDeclarationKind::Struct { fields, .. } = signatures::data(checker.context, target)
    else {
        return Err(BodyTypeFailure::InternalFailure);
    };
    let mut seen = resources::repeated(fields.len(), false)?;
    // Copy planning validates the complete field set before any field type constraint.
    for initializer in initializers {
        let (field_name, _) = field_value(initializer);
        let Some(index) = fields.iter().position(|field| field.name.text == field_name.text) else {
            legacy_error(
                checker,
                initializer.span,
                format!("struct '{}' has no field '{}'", name.text, field_name.text),
                "initialize exactly the declared field set",
            );
            return Ok(false);
        };
        if seen[index] {
            legacy_error(
                checker,
                initializer.span,
                format!("field '{}' is initialized more than once", field_name.text),
                "initialize every declared field exactly once",
            );
            return Ok(false);
        }
        seen[index] = true;
    }
    // Presence and value checks then follow the original declaration order.
    for field in fields {
        let Some(initializer) = initializers
            .iter()
            .find(|initializer| field_value(initializer).0.text == field.name.text)
        else {
            legacy_error(
                checker,
                at,
                format!("field '{}' is not initialized", field.name.text),
                "initialize every declared field exactly once",
            );
            return Ok(false);
        };
        let actual = checker.expression(owner, field_value(initializer).1);
        if !constraints::require(
            checker,
            owner,
            Some(substitution::source(target, field.type_syntax, 0)),
            actual,
            initializer.span,
            "struct field",
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}
