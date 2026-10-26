//! Symbolic declared children; opaque applications never acquire closed capabilities.
use super::super::{Closed, Failure, Originals, charge, copy_key, description, key, reserve};
use super::Projection;
fn description<'s>(
    originals: &Originals<'s>,
    bytes: &[u8],
    spent: &mut usize,
) -> Result<super::super::super::Description<'s>, Failure> {
    let shape = key::shape(bytes, spent)?;
    charge(spent, shape.children.len())?;
    let mut args = reserve(shape.children.len())?;
    for child in &shape.children {
        args.push(copy_key(child, spent)?);
    }
    description::identity(originals, shape.module, shape.index, &args, spent)
}
pub(super) fn arity(
    originals: &Originals<'_>,
    ty: &Closed,
    spent: &mut usize,
) -> Result<(bool, usize), Failure> {
    let Closed::Stored(bytes) = ty else {
        return Err(Failure::InternalFailure);
    };
    let shape = key::shape(bytes, spent)?;
    match shape.tag {
        0x10 | 0x12 => Ok((true, description(originals, bytes, spent)?.members.len())),
        0x20 => Ok((false, usize::try_from(shape.length).map_err(|_| Failure::InternalFailure)?)),
        _ => Err(Failure::InternalFailure),
    }
}
pub(super) fn projected(
    originals: &Originals<'_>,
    ty: Closed,
    projection: Projection,
    spent: &mut usize,
) -> Result<Closed, Failure> {
    let Closed::Stored(bytes) = ty else {
        return Err(Failure::InternalFailure);
    };
    match projection {
        Projection::Field(ordinal) => {
            let mut desc = description(originals, &bytes, spent)?;
            if ordinal >= desc.members.len() {
                return Err(Failure::InternalFailure);
            }
            Ok(desc.members.swap_remove(ordinal).1.ok_or(Failure::InternalFailure)?)
        }
        Projection::EnumPayload(ordinal) => {
            let shape = key::shape(&bytes, spent)?;
            match shape.tag {
                0x11 | 0x13 => {
                    let mut desc = description(originals, &bytes, spent)?;
                    if desc.structure || ordinal >= desc.members.len() {
                        return Err(Failure::InternalFailure);
                    }
                    Ok(desc.members.swap_remove(ordinal).1.ok_or(Failure::InternalFailure)?)
                }
                0x14 | 0x15 => {
                    let index = if shape.tag == 0x14 {
                        if ordinal != 1 {
                            return Err(Failure::InternalFailure);
                        }
                        0
                    } else {
                        if ordinal > 1 {
                            return Err(Failure::InternalFailure);
                        }
                        ordinal
                    };
                    Ok(Closed::Stored(copy_key(
                        shape.children.get(index).ok_or(Failure::InternalFailure)?,
                        spent,
                    )?))
                }
                _ => Err(Failure::InternalFailure),
            }
        }
        Projection::Element(index) => {
            let shape = key::shape(&bytes, spent)?;
            if shape.tag != 0x20 || index >= u64::from(shape.length) {
                return Err(Failure::InternalFailure);
            }
            Ok(Closed::Stored(copy_key(
                shape.children.first().ok_or(Failure::InternalFailure)?,
                spent,
            )?))
        }
    }
}
