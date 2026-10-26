//! Closed nominal/compiler shape arity, ordinals and stored-type references.

use super::{Base, Failure, counts, declaration_index, nominal, raw, reject};

pub(super) fn shape(graph: &raw::Graph, node: &raw::TypeNode) -> Result<(), Failure> {
    let valid = |id: raw::NodeId| {
        usize::try_from(id.0)
            .ok()
            .is_some_and(|index| graph.types.get(index).is_some_and(|node| node.id == id))
    };
    if let Some((module, index, parameters, kind)) = nominal(&node.kind) {
        let declaration = &graph.declarations[declaration_index(graph, module, index)?];
        if declaration.kind != kind
            || declaration.parameters != parameters
            || node.span != Some(declaration.span)
            || declaration.members as usize != counts(&node.kind).0
        {
            return Err(reject(
                node.span,
                "ZRYNA-L3001",
                "nominal kind, arity or original span differs",
            ));
        }
    } else if node.span.is_some() {
        return Err(reject(
            node.span,
            "ZRYNA-L3001",
            "compiler family or structural type forged a nominal span",
        ));
    }
    match &node.kind {
        raw::TypeKind::Struct { arguments, fields, .. } => {
            arguments_shape(arguments, valid, node)?;
            fields_shape(fields, valid, node)?;
        }
        raw::TypeKind::Enum { arguments, variants, .. } => {
            arguments_shape(arguments, valid, node)?;
            variants_shape(variants, valid, node)?;
        }
        raw::TypeKind::Option { argument } => {
            if !valid(*argument) {
                return Err(bad(node));
            }
        }
        raw::TypeKind::Result { okay, error } => {
            if !valid(*okay) || !valid(*error) {
                return Err(bad(node));
            }
        }
        raw::TypeKind::Base(kind) => match kind {
            Base::Struct { fields, .. } => fields_shape(fields, valid, node)?,
            Base::Enum { variants, .. } => variants_shape(variants, valid, node)?,
            Base::FixedArray { element, length } => {
                if !valid(*element) || *length > crate::MAX_ARRAY_LENGTH {
                    return Err(bad(node));
                }
            }
            Base::Vec { element }
            | Base::Shared { payload: element }
            | Base::Weak { payload: element } => {
                if !valid(*element) {
                    return Err(bad(node));
                }
            }
            Base::Borrow { .. } => {
                return Err(reject(
                    node.span,
                    "ZRYNA-L3004",
                    "borrow authorities have no stored successor layout",
                ));
            }
            Base::Bool | Base::I32 | Base::String => {}
        },
    }
    Ok(())
}

fn arguments_shape(
    arguments: &[raw::NodeId],
    valid: impl Fn(raw::NodeId) -> bool,
    node: &raw::TypeNode,
) -> Result<(), Failure> {
    if !(1..=2).contains(&arguments.len()) || arguments.iter().any(|id| !valid(*id)) {
        return Err(bad(node));
    }
    Ok(())
}

fn fields_shape(
    fields: &[raw::Field],
    valid: impl Fn(raw::NodeId) -> bool,
    node: &raw::TypeNode,
) -> Result<(), Failure> {
    if fields.is_empty()
        || fields.iter().enumerate().any(|(index, field)| {
            usize::try_from(field.ordinal).ok() != Some(index) || !valid(field.ty)
        })
    {
        return Err(bad(node));
    }
    Ok(())
}

fn variants_shape(
    variants: &[raw::Variant],
    valid: impl Fn(raw::NodeId) -> bool,
    node: &raw::TypeNode,
) -> Result<(), Failure> {
    if variants.is_empty()
        || variants.iter().enumerate().any(|(index, variant)| {
            usize::try_from(variant.ordinal).ok() != Some(index)
                || variant.payload.is_some_and(|id| !valid(id))
        })
    {
        return Err(bad(node));
    }
    Ok(())
}

fn bad(node: &raw::TypeNode) -> Failure {
    reject(
        node.span,
        "ZRYNA-L3003",
        "closed shape contains wrong arity, ordinal, length or type reference",
    )
}
