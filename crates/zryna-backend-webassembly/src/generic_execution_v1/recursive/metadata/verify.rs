//! Actual data is checked directly against original retained layout and Clone authorities.
use super::{Diagnostic, Model, TypeCategory, budget, category, clone_kind, invariant};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedCloneNode};
use zryna_layout::StorageTarget;
mod clones;

pub(in crate::generic_execution_v1) fn check(
    bytes: &[u8],
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let arena = m.memory.recursive_arena().ok_or_else(invariant)?;
    if bytes.len() != arena.metadata_bytes as usize {
        return Err(invariant());
    }
    m.take(bytes.len())?;
    let types = m.program.layouts(StorageTarget::Linear32V1).types();
    let count = types.len();
    m.take(count)?;
    let members =
        m.program.layouts(StorageTarget::Linear32V1).types().try_fold(0usize, |n, ty| {
            n.checked_add(ty.fields().len())
                .and_then(|v| v.checked_add(ty.variants().len()))
                .ok_or_else(budget)
        })?;
    let start =
        16usize.checked_add(count.checked_mul(32).ok_or_else(budget)?).ok_or_else(budget)?;
    need(bytes, 0, u32::try_from(count).map_err(|_| budget())?)?;
    need(bytes, 4, u32::try_from(members).map_err(|_| budget())?)?;
    need(
        bytes,
        8,
        arena
            .metadata_start
            .checked_add(u32::try_from(start).map_err(|_| budget())?)
            .ok_or_else(budget)?,
    )?;
    need(bytes, 12, 1)?;
    let grants = clones::derive(m, count)?;
    let mut at = 0usize;
    m.take(count)?;
    for (id, ty) in types.enumerate() {
        if id != ty.id().index() as usize {
            return Err(invariant());
        }
        let row = 16 + 32 * id;
        need(bytes, row, category(ty.category()))?;
        need(bytes, row + 4, u32::try_from(ty.size()).map_err(|_| budget())?)?;
        need(bytes, row + 8, u32::from(ty.drop_kind() != 0))?;
        need(bytes, row + 12, grants[id])?;
        check_type(bytes, m, ty, row, start, &mut at)?;
    }
    if at != members {
        return Err(invariant());
    }
    let end = start.checked_add(members.checked_mul(12).ok_or_else(budget)?).ok_or_else(budget)?;
    m.take(bytes.len().checked_sub(end).ok_or_else(invariant)?)?;
    if bytes[end..].iter().any(|b| *b != 0) {
        return Err(invariant());
    }
    Ok(())
}
fn check_type(
    bytes: &[u8],
    m: &Model<'_, '_, '_>,
    ty: zryna_layout::generic_v1::TypeView<'_>,
    row: usize,
    start: usize,
    at: &mut usize,
) -> Result<(), Diagnostic> {
    match ty.category() {
        TypeCategory::Struct | TypeCategory::Enum => {
            need(bytes, row + 16, u32::try_from(*at).map_err(|_| budget())?)?;
            if ty.category() == TypeCategory::Struct {
                m.take(ty.fields().len())?;
                need(bytes, row + 20, u32::try_from(ty.fields().len()).map_err(|_| budget())?)?;
                for (ordinal, child, offset) in ty.fields() {
                    member(
                        bytes,
                        start,
                        *at,
                        [ordinal, child.index(), u32::try_from(offset).map_err(|_| budget())?],
                    )?;
                    *at += 1;
                }
                need(bytes, row + 24, 0)?;
            } else {
                m.take(ty.variants().len())?;
                need(bytes, row + 20, u32::try_from(ty.variants().len()).map_err(|_| budget())?)?;
                let offset =
                    u32::try_from(ty.payload().map_or(0, |p| p.0)).map_err(|_| budget())?;
                need(bytes, row + 24, offset)?;
                for (ordinal, child) in ty.variants() {
                    member(
                        bytes,
                        start,
                        *at,
                        [
                            ordinal,
                            child.map_or(u32::MAX, zryna_layout::generic_v1::TypeId::index),
                            offset,
                        ],
                    )?;
                    *at += 1;
                }
            }
            need(bytes, row + 28, 0)?;
        }
        TypeCategory::FixedArray => {
            need(bytes, row + 16, ty.referenced_type().ok_or_else(invariant)?.index())?;
            need(
                bytes,
                row + 20,
                u32::try_from(ty.array_length().ok_or_else(invariant)?).map_err(|_| budget())?,
            )?;
            need(
                bytes,
                row + 24,
                u32::try_from(ty.array_stride().ok_or_else(invariant)?).map_err(|_| budget())?,
            )?;
            need(bytes, row + 28, 0)?;
        }
        TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
            let child = ty.referenced_type().ok_or_else(invariant)?.index();
            need(bytes, row + 16, child)?;
            need(bytes, row + 20, 0)?;
            if ty.category() == TypeCategory::Vec {
                let layouts = m.program.runtime().element_layouts();
                m.take(layouts.len())?;
                let layout = layouts
                    .iter()
                    .find(|r| {
                        r.target() == StorageTarget::Linear32V1 && r.element().index() == child
                    })
                    .ok_or_else(invariant)?;
                need(bytes, row + 24, u32::try_from(layout.stride()).map_err(|_| budget())?)?;
                need(bytes, row + 28, 0)?;
            } else {
                let layouts = m.program.runtime().control_layouts();
                m.take(layouts.len())?;
                let layout = layouts
                    .iter()
                    .find(|r| {
                        r.target() == StorageTarget::Linear32V1 && r.payload().index() == child
                    })
                    .ok_or_else(invariant)?;
                need(bytes, row + 24, 0)?;
                need(
                    bytes,
                    row + 28,
                    u32::try_from(layout.payload_offset()).map_err(|_| budget())?,
                )?;
            }
        }
        _ => {
            for offset in [16, 20, 24, 28] {
                need(bytes, row + offset, 0)?;
            }
        }
    }
    Ok(())
}
fn need(bytes: &[u8], at: usize, value: u32) -> Result<(), Diagnostic> {
    if bytes.get(at..at.checked_add(4).ok_or_else(budget)?) != Some(value.to_le_bytes().as_slice())
    {
        return Err(invariant());
    }
    Ok(())
}
fn member(bytes: &[u8], base: usize, index: usize, words: [u32; 3]) -> Result<(), Diagnostic> {
    let at = base.checked_add(index.checked_mul(12).ok_or_else(budget)?).ok_or_else(budget)?;
    for (i, value) in words.into_iter().enumerate() {
        need(bytes, at + 4 * i, value)?;
    }
    Ok(())
}
