//! Source declaration members, never layout drop metadata, define the closed `TypeWorld`.
use super::{
    Closed, Failure, Kind, Originals, RawDataDeclarationKind, Resolver, TypeWorld, WorldBuilder,
    charge, lookup, originals, reject, reserve,
};
use crate::generic_v1::keys::{self, Domain, Kind as KeyKind};
pub(super) fn closed(
    originals: &Originals<'_>,
    keys: &[Vec<u8>],
    spent: &mut usize,
) -> Result<TypeWorld, Failure> {
    let mut builder = WorldBuilder::closed(keys.len())?;
    for (index, key) in keys.iter().enumerate() {
        charge(spent, key.len().checked_mul(4).ok_or(Failure::InternalFailure)? + 67)?;
        let decoded = keys::decode(key, Domain::Type)?;
        let (module, declaration) = decoded.declaration().unwrap_or((0, 0));
        let arity =
            u32::try_from(decoded.arguments().len()).map_err(|_| Failure::InternalFailure)?;
        let kind = match decoded.kind() {
            KeyKind::Bool => Kind::Bool,
            KeyKind::I32 => Kind::I32,
            KeyKind::String => Kind::String,
            KeyKind::Struct | KeyKind::GenericStruct => Kind::Struct { module, declaration, arity },
            KeyKind::Enum | KeyKind::GenericEnum => Kind::Enum { module, declaration, arity },
            KeyKind::Option => Kind::Option,
            KeyKind::Result => Kind::Result,
            KeyKind::FixedArray(length) => Kind::FixedArray { length },
            KeyKind::Vec => Kind::Vec,
            KeyKind::Shared => Kind::Shared,
            KeyKind::Weak => Kind::Weak,
            _ => return Err(reject("function key in stored source world")),
        };
        charge(spent, decoded.arguments().len())?;
        let mut arguments = reserve(decoded.arguments().len())?;
        for key in decoded.arguments() {
            arguments.push(lookup(keys, key, spent)?);
        }
        let id = builder.declare(kind, &arguments, spent)?;
        if id as usize != index {
            return Err(reject("duplicate or mismatched complete source key"));
        }
        builder.set_key(id, key, spent)?;
    }
    for (index, key) in keys.iter().enumerate() {
        if !matches!(key.first(), Some(0x10..=0x13)) {
            continue;
        }
        charge(spent, key.len().checked_mul(4).ok_or(Failure::InternalFailure)? + 67)?;
        let decoded = keys::decode(key, Domain::Type)?;
        let (module, declaration) = decoded.declaration().ok_or(Failure::InternalFailure)?;
        charge(spent, 1)?;
        let data = originals
            .units
            .get(module as usize)
            .and_then(|u| u.data_declarations.get(declaration as usize))
            .ok_or_else(|| reject("source nominal declaration is absent"))?;
        let parameters = data.type_parameters.as_ref();
        if parameters.map_or(0, |p| p.parameters.len()) != decoded.arguments().len() {
            return Err(reject("nominal source arity differs"));
        }
        charge(spent, decoded.arguments().len())?;
        let mut arguments = reserve(decoded.arguments().len())?;
        arguments.extend(decoded.arguments());
        let resolver = Resolver { originals, module, parameters, arguments };
        let mut members = Vec::new();
        match &data.kind {
            RawDataDeclarationKind::Struct { fields, .. }
                if matches!(decoded.kind(), KeyKind::Struct | KeyKind::GenericStruct) =>
            {
                charge(spent, fields.len())?;
                members.try_reserve(fields.len()).map_err(|_| Failure::AllocationFailure)?;
                for field in fields {
                    members.push(Some(member(&resolver, field.type_syntax, keys, spent)?));
                }
            }
            RawDataDeclarationKind::Enum { variants, .. }
                if matches!(decoded.kind(), KeyKind::Enum | KeyKind::GenericEnum) =>
            {
                charge(spent, variants.len())?;
                members.try_reserve(variants.len()).map_err(|_| Failure::AllocationFailure)?;
                for variant in variants {
                    members.push(
                        variant
                            .payload_type
                            .map(|at| member(&resolver, at, keys, spent))
                            .transpose()?,
                    );
                }
            }
            _ => return Err(reject("nominal source kind differs from complete key")),
        }
        builder.set_members(
            u32::try_from(index).map_err(|_| Failure::InternalFailure)?,
            &members,
            spent,
        )?;
    }
    builder.seal(spent)
}
fn member(
    resolver: &Resolver<'_, '_>,
    at: u32,
    keys: &[Vec<u8>],
    spent: &mut usize,
) -> Result<u32, Failure> {
    originals::resolution_credit(resolver, at, spent)?;
    let Closed::Stored(key) = resolver.resolve_symbolic(at)? else {
        return Err(reject("source member cannot be unit or a stored loan"));
    };
    lookup(keys, &key, spent)
}
