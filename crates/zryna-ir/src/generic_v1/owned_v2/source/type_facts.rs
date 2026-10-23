//! Original-scoped enum facts; sealed layout ownership remains an independent proof.
use super::{Budget, Closed, owners};
use crate::generic_v1::{
    Failure, budget, reject, reserve,
    source::{Originals, Target},
    source_types::Resolver,
};
use zryna_syntax::v5::RawDataDeclarationKind;
mod declarations;
mod domain;
mod payloads;
pub(super) use declarations::check_declarations;
use domain::{add, argument, lane};
use payloads::Frame;

struct Fact {
    key: Vec<u8>,
    affine: bool,
    work: usize,
}
/// The cache belongs to one original body/opaque slot frame and never supplies layout authority.
#[derive(Default)]
pub(super) struct SourceTypeFacts {
    last: Option<Fact>,
}
impl SourceTypeFacts {
    pub(super) fn affine(
        &mut self,
        originals: &Originals<'_>,
        ty: &Closed,
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        let (Closed::Stored(key) | Closed::Borrow(key, _)) = ty else { return Ok(false) };
        if !matches!(key.first(), Some(0x11 | 0x13)) {
            owners::admitted(ty)?;
            return owners::affine(ty);
        }
        let affine = self.nominal(originals, key, credit)?;
        Ok(matches!(ty, Closed::Stored(_)) && affine)
    }
    pub(super) fn description(
        &mut self,
        resolver: &Resolver<'_, '_>,
        name: &str,
        arguments: &[Vec<u8>],
        credit: &mut Budget,
    ) -> Result<(), Failure> {
        if matches!(name, "Option" | "Result") {
            return Ok(());
        }
        let Target::Data(module, index) = resolver.originals.resolve(resolver.module, name)? else {
            return Err(reject("enum constructor names a function"));
        };
        if arguments.len() > 2 {
            return Err(reject("nominal enum has wrong generic arity"));
        }
        let borrowed = [
            arguments.first().map_or(&[][..], Vec::as_slice),
            arguments.get(1).map_or(&[][..], Vec::as_slice),
        ];
        let key = nominal_key(module, index, &borrowed[..arguments.len()], credit)?;
        self.nominal(resolver.originals, &key, credit)?;
        Ok(())
    }
    fn nominal(
        &mut self,
        originals: &Originals<'_>,
        key: &[u8],
        credit: &mut Budget,
    ) -> Result<bool, Failure> {
        // One comparison and the same complete logical proof are charged on every call.
        // Replacing the last row changes neither admissibility nor the successful cost.
        if key.len() > 4096 {
            return Err(budget("nominal complete key exceeds 4096 bytes"));
        }
        credit.nominal(add(1, key.len())?)?;
        if let Some(row) = self.last.as_ref().filter(|row| row.key == key) {
            credit.nominal(row.work)?;
            return Ok(row.affine);
        }
        // A failed incremental walk has the same visible credit/cache state as a failed
        // atomic hit charge. Successful analysis pays before retaining any copied key.
        let mut checked = *credit;
        let affine = analyze(originals, key, &mut checked)?;
        let work = checked
            .nominal_used()
            .checked_sub(credit.nominal_used())
            .ok_or(Failure::InternalFailure)?;
        let mut retained = reserve(key.len())?;
        retained.extend_from_slice(key);
        self.last = Some(Fact { key: retained, affine, work });
        *credit = checked;
        Ok(affine)
    }
}

fn analyze(originals: &Originals<'_>, key: &[u8], credit: &mut Budget) -> Result<bool, Failure> {
    if key.len() > 4096 {
        return Err(budget("nominal complete key exceeds 4096 bytes"));
    }
    // Bound the complete key/argument scan and all key retention copies before traversal.
    credit.nominal(key.len().checked_mul(3).ok_or(Failure::InternalFailure)?)?;
    let mut cursor = 1;
    let module = lane(key, &mut cursor)?;
    let index = lane(key, &mut cursor)?;
    let original = originals
        .units
        .get(module as usize)
        .and_then(|unit| unit.data_declarations.get(index as usize))
        .ok_or_else(|| reject("nominal source key names an unknown original declaration"))?;
    let RawDataDeclarationKind::Enum { variants, .. } = &original.kind else {
        return Err(reject("owned nominal source lane requires an original enum"));
    };
    let count = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
    match (key.first(), count) {
        (Some(0x11), 0) => {}
        (Some(0x13), 1..=2) if lane(key, &mut cursor)? as usize == count => {}
        _ => return Err(reject("nominal key kind/arity differs from its original declaration")),
    }
    let mut arguments = [&[][..]; 2];
    for child in &mut arguments[..count] {
        let length = lane(key, &mut cursor)? as usize;
        let end = add(cursor, length)?;
        *child = key.get(cursor..end).ok_or(Failure::InternalFailure)?;
        let shape = argument(child, 0)?;
        if add(shape.depth, 1)? > 64 {
            return Err(budget("nominal complete key depth exceeds 64"));
        }
        cursor = end;
    }
    if cursor != key.len() {
        return Err(Failure::InternalFailure);
    }
    if variants.len() > zryna_layout::MAX_MEMBERS_PER_DECLARATION {
        return Err(budget("nominal original member count exceeds 1024"));
    }
    let frame = Frame { originals, module, declaration: original, arguments: &arguments[..count] };
    let mut affine = false;
    // Cache key, description key, argument descriptors and the entire variant inventory.
    credit.nominal(add(count, variants.len())?)?;
    for variant in variants {
        if let Some(occurrence) = variant.payload_type {
            let shape = frame.shape(occurrence, 0, credit)?.checked()?;
            affine |= shape.affine;
            credit.nominal(shape.bytes.checked_mul(3).ok_or(Failure::InternalFailure)?)?;
        }
    }
    Ok(affine)
}

fn nominal_key(
    module: u32,
    index: u32,
    arguments: &[&[u8]],
    credit: &mut Budget,
) -> Result<Vec<u8>, Failure> {
    let tag = if arguments.is_empty() { 0x11 } else { 0x13 };
    let mut length = if arguments.is_empty() { 9 } else { 13 };
    for key in arguments {
        length = add(length, add(4, key.len())?)?;
    }
    if length > 4096 {
        return Err(budget("nominal complete key exceeds 4096 bytes"));
    }
    credit.nominal(length)?;
    for key in arguments {
        argument(key, 0)?;
    }
    let mut key = reserve(length)?;
    key.push(tag);
    key.extend_from_slice(&module.to_le_bytes());
    key.extend_from_slice(&index.to_le_bytes());
    if !arguments.is_empty() {
        key.extend_from_slice(
            &u32::try_from(arguments.len()).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
        );
        for argument in arguments {
            key.extend_from_slice(
                &u32::try_from(argument.len()).map_err(|_| Failure::InternalFailure)?.to_le_bytes(),
            );
            key.extend_from_slice(argument);
        }
    }
    Ok(key)
}

#[cfg(test)]
mod tests;
