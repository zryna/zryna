//! Members substitute in their declaring module, preserving caller opaque identities.
use super::{
    Closed, Failure, Originals, RawDataDeclarationKind, Resolver, Target, charge, copy_key, key,
    reject, reserve, resolution, resolve, small,
};
pub(in crate::generic_v1::execution_v1::source) struct Description<'s> {
    pub ty: Closed,
    pub structure: bool,
    pub members: Vec<(&'s str, Option<Closed>)>,
}
pub(in crate::generic_v1::execution_v1::source) fn describe<'s>(
    resolver: &Resolver<'_, 's>,
    name: &str,
    arguments: &[Vec<u8>],
    spent: &mut usize,
) -> Result<Description<'s>, Failure> {
    charge(spent, name.len())?;
    match name {
        "Option" | "Result" => family(name, arguments, spent),
        _ => {
            resolution::name_credit(resolver, name, spent)?;
            let Target::Data(module, index) = resolver.originals.resolve(resolver.module, name)?
            else {
                return Err(reject("source constructor names a function"));
            };
            identity(resolver.originals, module, index, arguments, spent)
        }
    }
}
pub(super) fn identity<'s>(
    originals: &Originals<'s>,
    module: u32,
    index: u32,
    arguments: &[Vec<u8>],
    spent: &mut usize,
) -> Result<Description<'s>, Failure> {
    charge(spent, 1)?;
    let original = originals
        .units
        .get(module as usize)
        .and_then(|unit| unit.data_declarations.get(index as usize))
        .ok_or_else(|| reject("source nominal declaration is absent"))?;
    let arity = original.type_parameters.as_ref().map_or(0, |p| p.parameters.len());
    if arity > 2 || arguments.len() != arity {
        return Err(reject("source nominal requires exact generic arity"));
    }
    charge(spent, arity)?;
    let mut args = reserve(arity)?;
    args.extend(arguments.iter().map(Vec::as_slice));
    let substitution = Resolver {
        originals,
        module,
        parameters: original.type_parameters.as_ref(),
        arguments: args,
    };
    let mut members = Vec::new();
    let structure = matches!(original.kind, RawDataDeclarationKind::Struct { .. });
    match &original.kind {
        RawDataDeclarationKind::Struct { fields, .. } => {
            charge(spent, fields.len())?;
            members.try_reserve(fields.len()).map_err(|_| Failure::AllocationFailure)?;
            for field in fields {
                let ty = resolve(&substitution, field.type_syntax, spent)?;
                stored(&ty)?;
                members.push((field.name.text.as_str(), Some(ty)));
            }
        }
        RawDataDeclarationKind::Enum { variants, .. } => {
            charge(spent, variants.len())?;
            members.try_reserve(variants.len()).map_err(|_| Failure::AllocationFailure)?;
            for variant in variants {
                let ty =
                    variant.payload_type.map(|at| resolve(&substitution, at, spent)).transpose()?;
                if let Some(ty) = &ty {
                    stored(ty)?;
                }
                members.push((variant.name.text.as_str(), ty));
            }
        }
    }
    let tag = match (structure, arity) {
        (true, 0) => 0x10,
        (false, 0) => 0x11,
        (true, _) => 0x12,
        (false, _) => 0x13,
    };
    let mut lanes = [module, index, small(arity)?];
    let lanes = if arity == 0 { &mut lanes[..2] } else { &mut lanes[..] };
    Ok(Description { ty: key::encode(tag, lanes, arguments, spent)?, structure, members })
}
fn family<'s>(name: &str, args: &[Vec<u8>], spent: &mut usize) -> Result<Description<'s>, Failure> {
    let count = if name == "Option" { 1 } else { 2 };
    if args.len() != count {
        return Err(reject("compiler family requires exact generic arity"));
    }
    charge(spent, 2)?;
    let mut members = reserve(2)?;
    if name == "Option" {
        members.push(("none", None));
        members.push(("some", Some(Closed::Stored(copy_key(&args[0], spent)?))));
    } else {
        members.push(("ok", Some(Closed::Stored(copy_key(&args[0], spent)?))));
        members.push(("err", Some(Closed::Stored(copy_key(&args[1], spent)?))));
    }
    Ok(Description {
        ty: key::encode(if count == 1 { 0x14 } else { 0x15 }, &[small(count)?], args, spent)?,
        structure: false,
        members,
    })
}
fn stored(ty: &Closed) -> Result<(), Failure> {
    if !matches!(ty, Closed::Stored(_)) {
        return Err(reject("original member must be a stored value"));
    }
    Ok(())
}
