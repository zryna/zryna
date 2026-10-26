//! Original/compiler enum descriptions and complete generic-key argument decoding.

use super::Description;
use crate::generic_v1::{
    Failure, reject, reserve,
    source::Target,
    source_types::{Closed, Resolver, encode_mode},
};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(in super::super) fn describe<'a>(
    resolver: &Resolver<'_, 'a>,
    name: &str,
    arguments: &[Vec<u8>],
    symbolic: bool,
) -> Result<Description<'a>, Failure> {
    let mut variants = reserve(2)?;
    let (tag, lanes) = match name {
        "Option" if arguments.len() == 1 => {
            variants.extend([("none", None), ("some", Some(Closed::Stored(arguments[0].clone())))]);
            (0x14, vec![1])
        }
        "Result" if arguments.len() == 2 => {
            variants.extend([
                ("ok", Some(Closed::Stored(arguments[0].clone()))),
                ("err", Some(Closed::Stored(arguments[1].clone()))),
            ]);
            (0x15, vec![2])
        }
        "Option" | "Result" => {
            return Err(reject("compiler enum family requires exact explicit arity"));
        }
        _ => {
            let Target::Data(module, index) = resolver.originals.resolve(resolver.module, name)?
            else {
                return Err(reject("enum constructor names a function"));
            };
            let original =
                &resolver.originals.units[module as usize].data_declarations[index as usize];
            let RawDataDeclarationKind::Enum { variants: declared, .. } = &original.kind else {
                return Err(reject("enum constructor names a struct"));
            };
            let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
            if arity != arguments.len() {
                return Err(reject("nominal enum has wrong generic arity"));
            }
            let mut keys = reserve(arity)?;
            keys.extend(arguments.iter().map(Vec::as_slice));
            let substitution = Resolver {
                originals: resolver.originals,
                module,
                parameters: original.type_parameters.as_ref(),
                arguments: keys,
            };
            variants.try_reserve_exact(declared.len()).map_err(|_| Failure::AllocationFailure)?;
            for variant in declared {
                let payload = variant
                    .payload_type
                    .map(|occurrence| substitution.resolve_symbolic(occurrence))
                    .transpose()?;
                if payload.as_ref().is_some_and(|ty| !matches!(ty, Closed::Stored(_))) {
                    return Err(reject("enum payload is not a stored value"));
                }
                variants.push((&variant.name.text, payload));
            }
            if arity == 0 {
                (0x11, vec![module, index])
            } else {
                (
                    0x13,
                    vec![
                        module,
                        index,
                        u32::try_from(arity).map_err(|_| Failure::InternalFailure)?,
                    ],
                )
            }
        }
    };
    Ok(Description { ty: Closed::Stored(encode_mode(tag, &lanes, arguments, symbolic)?), variants })
}

pub(super) fn arguments(key: &[u8]) -> Result<Vec<Vec<u8>>, Failure> {
    let (offset, count) = match key.first() {
        Some(0x14) => (5, 1),
        Some(0x15) => (5, 2),
        Some(0x11) => (9, 0),
        Some(0x13) => (
            13,
            u32::from_le_bytes(
                key.get(9..13)
                    .ok_or(Failure::InternalFailure)?
                    .try_into()
                    .map_err(|_| Failure::InternalFailure)?,
            ) as usize,
        ),
        _ => return Err(reject("source match requires an original or compiler-owned enum")),
    };
    let mut result = reserve(count)?;
    let mut cursor = offset;
    for _ in 0..count {
        let length = u32::from_le_bytes(
            key.get(cursor..cursor + 4)
                .ok_or(Failure::InternalFailure)?
                .try_into()
                .map_err(|_| Failure::InternalFailure)?,
        ) as usize;
        cursor += 4;
        let end = cursor.checked_add(length).ok_or(Failure::InternalFailure)?;
        let mut child = reserve(length)?;
        child.extend_from_slice(key.get(cursor..end).ok_or(Failure::InternalFailure)?);
        result.push(child);
        cursor = end;
    }
    if cursor != key.len() {
        return Err(Failure::InternalFailure);
    }
    Ok(result)
}
