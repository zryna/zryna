use std::collections::BTreeMap;
use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawIdentifierSyntax;

use super::{Binding, Errors, OwnerState, Ty};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StringBytes {
    Known(u64),
    Unknown,
}

impl StringBytes {
    pub(super) fn from_known(value: Option<u64>) -> Self {
        value.map_or(Self::Unknown, Self::Known)
    }

    pub(super) fn known(self) -> Option<u64> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown => None,
        }
    }
}

pub(super) fn concat_optional_bytes(
    left: Option<u64>,
    right: Option<u64>,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<StringBytes> {
    match (left, right) {
        (Some(left), Some(right)) => concat_bytes(left, right, at, errors).map(StringBytes::Known),
        _ => Some(StringBytes::Unknown),
    }
}

pub(super) fn concat_arguments(
    arguments: &[u32],
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<[u32; 2]> {
    let [left, right] = arguments else {
        errors.at(
            "ZRYNA-M3012",
            at,
            "String concat requires exactly two operands",
            "call concat(left, right) with two available String values",
        );
        return None;
    };
    Some([*left, *right])
}

pub(super) fn concat_bytes(
    left: u64,
    right: u64,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<u64> {
    super::global_resource_limits::checked_string_concat_bytes(left, right).or_else(|| {
        errors.at(
            "ZRYNA-M3012",
            at,
            "String concatenation exceeds the sealed runtime byte limit",
            "reduce the statically known concatenated String size",
        );
        None
    })
}

// Read-only local selection shared by the existing Vec evaluator and mixed preparation.
// Compound operands are handled by their caller's ordered expression authority.
pub(super) fn local(
    name: &RawIdentifierSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    known_bytes: &BTreeMap<raw::PlaceId, u64>,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<(raw::PlaceId, u64)> {
    let (place, bytes) = local_source(name, bindings, owners, known_bytes, None, at, errors)?;
    Some((place, bytes.known()?))
}

pub(super) fn local_source(
    name: &RawIdentifierSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    known_bytes: &BTreeMap<raw::PlaceId, u64>,
    expected: Option<Ty>,
    at: Span,
    errors: &mut Errors<'_>,
) -> Option<(raw::PlaceId, StringBytes)> {
    let Some(binding) = bindings.get(&name.text) else {
        errors.at(
            "ZRYNA-M3002",
            at,
            format!("String operand '{}' is not declared", name.text),
            "reference one exact preceding String local",
        );
        return None;
    };
    if binding.ty.category != TypeCategory::String || expected.is_some_and(|ty| ty != binding.ty) {
        errors.at(
            "ZRYNA-M3012",
            at,
            "String operand has the wrong exact type",
            "use one exact String value",
        );
        return None;
    }
    if !owners.contains(binding.place) {
        errors.at(
            "ZRYNA-M3014",
            at,
            format!("String value '{}' was already moved", name.text),
            "use each owned String only while it remains available",
        );
        return None;
    }
    Some((binding.place, StringBytes::from_known(known_bytes.get(&binding.place).copied())))
}
