//! Bounded scalar/family arguments for the first original nominal enum proof.
use super::super::Budget;
use crate::generic_v1::{Failure, budget, reject};

#[derive(Clone, Copy)]
pub(super) struct Shape {
    pub bytes: usize,
    pub depth: usize,
    pub affine: bool,
    pub work: usize,
}
impl Shape {
    pub fn leaf(bytes: usize, affine: bool) -> Self {
        Self { bytes, depth: 0, affine, work: bytes + 1 }
    }
    pub fn family(children: &[Self]) -> Result<Self, Failure> {
        let mut result = Self { bytes: 5, depth: 1, affine: false, work: 1 };
        for child in children {
            result.bytes = add(result.bytes, add(4, child.bytes)?)?;
            result.depth = result.depth.max(add(1, child.depth)?);
            result.affine |= child.affine;
            result.work = add(result.work, child.work)?;
        }
        result.checked()
    }
    pub fn checked(self) -> Result<Self, Failure> {
        if self.bytes > 4096 || self.depth > 64 {
            return Err(budget("nominal substituted key exceeds inherited byte/depth ceiling"));
        }
        Ok(self)
    }
}

pub(super) fn add(left: usize, right: usize) -> Result<usize, Failure> {
    left.checked_add(right).ok_or_else(|| budget("nominal source type work overflow"))
}
pub(super) fn lane(key: &[u8], cursor: &mut usize) -> Result<u32, Failure> {
    let end = add(*cursor, 4)?;
    let bytes = key.get(*cursor..end).ok_or(Failure::InternalFailure)?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| Failure::InternalFailure)?))
}

pub(super) fn argument(key: &[u8], depth: usize) -> Result<Shape, Failure> {
    walk(key, depth, false, &mut |_| Ok(()))
}

pub(super) fn charged_argument(key: &[u8], credit: &mut Budget) -> Result<Shape, Failure> {
    // The referring source AST node already paid the root visit.
    walk(key, 0, true, &mut |units| credit.nominal(units))
}

fn walk(
    key: &[u8],
    depth: usize,
    root_paid: bool,
    charge: &mut impl FnMut(usize) -> Result<(), Failure>,
) -> Result<Shape, Failure> {
    if !root_paid {
        charge(1)?;
    }
    if key.len() > 4096 || depth > 64 {
        return Err(budget("nominal substituted key exceeds inherited byte/depth ceiling"));
    }
    let count = match key.first() {
        Some(0 | 1) if key.len() == 1 => {
            charge(1)?;
            return Ok(Shape::leaf(1, false));
        }
        Some(2 | 0x30 | 0x31) if key.len() == 1 => {
            charge(1)?;
            return Ok(Shape::leaf(1, true));
        }
        Some(0x14) => 1,
        Some(0x15) => 2,
        _ => return Err(reject("nominal payload/argument requires scalar/String/Option/Result")),
    };
    let mut cursor = 1;
    if lane(key, &mut cursor)? != count {
        return Err(reject("nominal argument has an inexact compiler-family arity"));
    }
    let mut children = [Shape::leaf(0, false); 2];
    for child in &mut children[..count as usize] {
        let length = lane(key, &mut cursor)? as usize;
        let end = add(cursor, length)?;
        *child =
            walk(key.get(cursor..end).ok_or(Failure::InternalFailure)?, depth + 1, false, charge)?;
        cursor = end;
    }
    if cursor != key.len() {
        return Err(Failure::InternalFailure);
    }
    Shape::family(&children[..count as usize])
}
