//! Bounded symbolic key readers; opaque tags stay within their actual owner frame.
use super::{Closed, Failure, add, charge, exhausted, push, reject, reserve};
pub(super) struct Shape<'a> {
    pub tag: u8,
    pub module: u32,
    pub index: u32,
    pub length: u32,
    pub children: Vec<&'a [u8]>,
}
pub(super) fn shape<'a>(key: &'a [u8], spent: &mut usize) -> Result<Shape<'a>, Failure> {
    if key.is_empty() || key.len() > 4096 {
        return Err(reject("original symbolic key is empty or oversized"));
    }
    charge(spent, key.len())?;
    let tag = key[0];
    let (mut cursor, mut module, mut index, mut length) = (1, 0, 0, 0);
    let count = match tag {
        0..=2 | 0x30..=0x31 => 0,
        0x10 | 0x11 => {
            module = lane(key, &mut cursor)?;
            index = lane(key, &mut cursor)?;
            0
        }
        0x12 | 0x13 => {
            module = lane(key, &mut cursor)?;
            index = lane(key, &mut cursor)?;
            lane(key, &mut cursor)?
        }
        0x14 => {
            if lane(key, &mut cursor)? != 1 {
                return Err(reject("Option source arity"));
            }
            1
        }
        0x15 => {
            if lane(key, &mut cursor)? != 2 {
                return Err(reject("Result source arity"));
            }
            2
        }
        0x20 => {
            length = lane(key, &mut cursor)?;
            1
        }
        0x21..=0x23 => 1,
        _ => return Err(reject("unknown original symbolic type constructor")),
    };
    if count > 2 {
        return Err(reject("original symbolic constructor arity exceeds two"));
    }
    let count = usize::try_from(count).map_err(|_| Failure::InternalFailure)?;
    charge(spent, count)?;
    let mut children = reserve(count)?;
    for _ in 0..count {
        let bytes =
            usize::try_from(lane(key, &mut cursor)?).map_err(|_| Failure::InternalFailure)?;
        let end = add(cursor, bytes)?;
        children.push(key.get(cursor..end).ok_or_else(|| reject("short symbolic child key"))?);
        cursor = end;
    }
    if cursor != key.len() {
        return Err(reject("trailing symbolic type key bytes"));
    }
    Ok(Shape { tag, module, index, length, children })
}
fn lane(key: &[u8], cursor: &mut usize) -> Result<u32, Failure> {
    let end = add(*cursor, 4)?;
    let bytes = key.get(*cursor..end).ok_or_else(|| reject("short symbolic key lane"))?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| Failure::InternalFailure)?))
}
pub(super) fn encode(
    tag: u8,
    lanes: &[u32],
    args: &[Vec<u8>],
    spent: &mut usize,
) -> Result<Closed, Failure> {
    let mut size = add(1, lanes.len().checked_mul(4).ok_or(Failure::InternalFailure)?)?;
    for arg in args {
        size = add(size, add(4, arg.len())?)?;
    }
    charge(spent, add(size, args.len())?)?;
    Ok(Closed::Stored(crate::generic_v1::source_types::encode_mode(tag, lanes, args, true)?))
}

/// Inherited complete-key depth is independent of nominal member discovery depth.
pub(super) fn validate(bytes: &[u8], spent: &mut usize) -> Result<(), Failure> {
    if bytes.len() > 4096 {
        return Err(exhausted("original complete key exceeds inherited 4096 bytes"));
    }
    let mut pending = Vec::new();
    push(&mut pending, (bytes, 0u32), spent)?;
    while let Some((bytes, parent)) = pending.pop() {
        charge(spent, 1)?;
        let shape = shape(bytes, spent)?;
        let depth = parent
            .checked_add(u32::from(!shape.children.is_empty()))
            .ok_or(Failure::InternalFailure)?;
        if depth > 64 {
            return Err(exhausted("original complete key exceeds inherited application depth64"));
        }
        for child in shape.children {
            push(&mut pending, (child, depth), spent)?;
        }
    }
    Ok(())
}
