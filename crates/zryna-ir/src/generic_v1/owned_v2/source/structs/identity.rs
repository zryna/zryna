//! Exact original Struct identity and declaration-order substituted stored fields.
use super::super::{Admission, Budget};
use crate::generic_v1::{
    Failure, reject, reserve,
    source::Target,
    source_types::{Closed, Resolver, encode_mode},
};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(in super::super) struct Description<'a> {
    pub ty: Closed,
    pub fields: Vec<(&'a str, Closed)>,
}

pub(in super::super) fn describe<'a>(
    resolver: &Resolver<'_, 'a>,
    name: &str,
    arguments: &[Vec<u8>],
    symbolic: bool,
    admission: Admission,
) -> Result<Description<'a>, Failure> {
    if admission != Admission::StructV4 {
        return Err(reject("Struct source operations require the distinct private v4 issuer"));
    }
    let Target::Data(module, index) = resolver.originals.resolve(resolver.module, name)? else {
        return Err(reject("Struct constructor names an original function"));
    };
    describe_identity(resolver, module, index, arguments, symbolic)
}

fn describe_identity<'a>(
    resolver: &Resolver<'_, 'a>,
    module: u32,
    index: u32,
    arguments: &[Vec<u8>],
    symbolic: bool,
) -> Result<Description<'a>, Failure> {
    let original = &resolver.originals.units[module as usize].data_declarations[index as usize];
    let RawDataDeclarationKind::Struct { fields, .. } = &original.kind else {
        return Err(reject("Struct operation requires an original Struct declaration"));
    };
    let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
    if arity != arguments.len() || arity > 2 || fields.is_empty() {
        return Err(reject("Struct requires exact generic arity and nonempty original fields"));
    }
    let mut keys = reserve(arity)?;
    keys.extend(arguments.iter().map(Vec::as_slice));
    let substitution = Resolver {
        originals: resolver.originals,
        module,
        parameters: original.type_parameters.as_ref(),
        arguments: keys,
    };
    let mut members = reserve(fields.len())?;
    for field in fields {
        let ty = substitution.resolve_symbolic(field.type_syntax)?;
        if !matches!(ty, Closed::Stored(_)) {
            return Err(reject("Struct fields must be original stored value types"));
        }
        members.push((field.name.text.as_str(), ty));
    }
    let mut lanes = reserve(3)?;
    lanes.extend([module, index]);
    let tag = if arity == 0 {
        0x10
    } else {
        lanes.push(u32::try_from(arity).map_err(|_| Failure::InternalFailure)?);
        0x12
    };
    Ok(Description {
        ty: Closed::Stored(encode_mode(tag, &lanes, arguments, symbolic)?),
        fields: members,
    })
}

pub(in super::super) fn stored<'a>(
    resolver: &Resolver<'_, 'a>,
    key: &[u8],
    symbolic: bool,
    admission: Admission,
    credit: &mut Budget,
) -> Result<Description<'a>, Failure> {
    if admission != Admission::StructV4 || !matches!(key.first(), Some(0x10 | 0x12)) {
        return Err(reject("field observation requires a complete stored Struct in private v4"));
    }
    // Decode and retain children only after charging all header/argument key copies.
    credit.nominal(key.len())?;
    let mut cursor = 1;
    let module = lane(key, &mut cursor)?;
    let index = lane(key, &mut cursor)?;
    let count = if key[0] == 0x12 { lane(key, &mut cursor)? as usize } else { 0 };
    if count > 2 {
        return Err(reject("Struct field base has inexact original arity"));
    }
    let mut arguments = reserve(count)?;
    for _ in 0..count {
        let n = lane(key, &mut cursor)? as usize;
        let end = cursor.checked_add(n).ok_or(Failure::InternalFailure)?;
        let bytes = key.get(cursor..end).ok_or(Failure::InternalFailure)?;
        let mut argument = reserve(n)?;
        argument.extend_from_slice(bytes);
        arguments.push(argument);
        cursor = end;
    }
    if cursor != key.len() {
        return Err(Failure::InternalFailure);
    }
    let description = describe_identity(resolver, module, index, &arguments, symbolic)?;
    if !matches!(&description.ty, Closed::Stored(expected) if expected.as_slice() == key) {
        return Err(reject("Struct field base differs from its exact original identity"));
    }
    Ok(description)
}

fn lane(key: &[u8], cursor: &mut usize) -> Result<u32, Failure> {
    let end = cursor.checked_add(4).ok_or(Failure::InternalFailure)?;
    let bytes = key.get(*cursor..end).ok_or(Failure::InternalFailure)?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| Failure::InternalFailure)?))
}
