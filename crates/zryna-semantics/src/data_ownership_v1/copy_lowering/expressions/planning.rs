use std::collections::BTreeMap;

use zryna_source::{SourceMap, Span};
use zryna_syntax::v4::{self as syntax, RawFieldInitializerKind};

use crate::data_ownership_v1::{Errors, span};

pub(super) fn struct_initializers(
    sources: &SourceMap,
    name: &syntax::RawIdentifierSyntax,
    fields: &[syntax::RawFieldInitializer],
    declared: &[syntax::RawDataField],
    errors: &mut Errors<'_>,
) -> Option<BTreeMap<String, (u32, Span)>> {
    let mut initializers = BTreeMap::new();
    for field in fields {
        let (field_name, expression) = match &field.kind {
            RawFieldInitializerKind::Shorthand { name, value }
            | RawFieldInitializerKind::Explicit { name, value, .. } => (&name.text, *value),
        };
        if declared.iter().all(|candidate| candidate.name.text != *field_name) {
            errors.at(
                "ZRYNA-M3005",
                span(sources, field.span),
                format!("struct '{}' has no field '{field_name}'", name.text),
                "initialize exactly the declared field set",
            );
            return None;
        }
        if initializers
            .insert(field_name.clone(), (expression, span(sources, field.span)))
            .is_some()
        {
            errors.at(
                "ZRYNA-M3005",
                span(sources, field.span),
                format!("field '{field_name}' is initialized more than once"),
                "initialize every declared field exactly once",
            );
            return None;
        }
    }
    Some(initializers)
}
