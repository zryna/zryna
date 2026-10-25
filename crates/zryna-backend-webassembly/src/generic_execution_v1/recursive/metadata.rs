//! Immutable physical rows; Clone flags come only from retained verified Clone nodes.
use super::super::{Diagnostic, budget, invariant, model::Model};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
mod verify;

pub(in crate::generic_execution_v1) fn encode(
    m: &Model<'_, '_, '_>,
) -> Result<Vec<u8>, Diagnostic> {
    let arena = m.memory.recursive_arena().ok_or_else(invariant)?;
    let n = m.types.len();
    m.take(n)?;
    let members = m.types.iter().try_fold(0usize, |total, ty| {
        total
            .checked_add(ty.fields().len())
            .and_then(|v| v.checked_add(ty.variants().len()))
            .ok_or_else(budget)
    })?;
    let bytes = usize::try_from(arena.metadata_bytes).map_err(|_| budget())?;
    m.take(bytes.checked_mul(3).ok_or_else(budget)?)?;
    let mut output = Vec::new();
    output.try_reserve_exact(bytes).map_err(|_| budget())?;
    output.resize(bytes, 0);
    let entry = 16usize.checked_add(n.checked_mul(32).ok_or_else(budget)?).ok_or_else(budget)?;
    header(&mut output, arena.metadata_start, n, members, entry)?;
    let mut member = 0usize;
    m.take(n)?;
    for (id, ty) in m.types.iter().enumerate() {
        if ty.id().index() as usize != id {
            return Err(invariant());
        }
        let mut words = [0u32; 8];
        words[0] = category(ty.category());
        words[1] = u32::try_from(ty.size()).map_err(|_| budget())?;
        words[2] = u32::from(ty.drop_kind() != 0);
        words[3] = m.clones[id].map_or(0, |node| clone_kind(node.kind()));
        match ty.category() {
            TypeCategory::Struct | TypeCategory::Enum => {
                words[4] = u32::try_from(member).map_err(|_| budget())?;
                let count = if ty.category() == TypeCategory::Struct {
                    ty.fields().len()
                } else {
                    ty.variants().len()
                };
                words[5] = u32::try_from(count).map_err(|_| budget())?;
                m.take(count)?;
                if ty.category() == TypeCategory::Struct {
                    for (ordinal, child, offset) in ty.fields() {
                        entry_store(
                            &mut output,
                            entry,
                            member,
                            [ordinal, child.index(), u32::try_from(offset).map_err(|_| budget())?],
                        )?;
                        member += 1;
                    }
                } else {
                    let offset = ty.payload().map_or(0, |p| p.0);
                    words[6] = u32::try_from(offset).map_err(|_| budget())?;
                    for (ordinal, child) in ty.variants() {
                        entry_store(
                            &mut output,
                            entry,
                            member,
                            [
                                ordinal,
                                child.map_or(u32::MAX, zryna_layout::generic_v1::TypeId::index),
                                words[6],
                            ],
                        )?;
                        member += 1;
                    }
                }
            }
            TypeCategory::FixedArray => {
                words[4] = ty.referenced_type().ok_or_else(invariant)?.index();
                words[5] = u32::try_from(ty.array_length().ok_or_else(invariant)?)
                    .map_err(|_| budget())?;
                words[6] = u32::try_from(ty.array_stride().ok_or_else(invariant)?)
                    .map_err(|_| budget())?;
            }
            TypeCategory::Vec => {
                words[4] = ty.referenced_type().ok_or_else(invariant)?.index();
                words[6] =
                    u32::try_from(m.element(ty.id().index())?.stride()).map_err(|_| budget())?;
            }
            TypeCategory::Shared | TypeCategory::Weak => {
                words[4] = ty.referenced_type().ok_or_else(invariant)?.index();
                words[7] = u32::try_from(m.control(ty.id().index())?.payload_offset())
                    .map_err(|_| budget())?;
            }
            _ => {}
        }
        for (word, value) in words.into_iter().enumerate() {
            store(&mut output, 16 + id * 32 + word * 4, value)?;
        }
    }
    if member != members {
        return Err(invariant());
    }
    Ok(output)
}
fn header(
    output: &mut [u8],
    metadata_start: u32,
    n: usize,
    members: usize,
    entry: usize,
) -> Result<(), Diagnostic> {
    for (at, value) in [
        (0, u32::try_from(n).map_err(|_| budget())?),
        (4, u32::try_from(members).map_err(|_| budget())?),
        (
            8,
            metadata_start
                .checked_add(u32::try_from(entry).map_err(|_| budget())?)
                .ok_or_else(budget)?,
        ),
        (12, 1),
    ] {
        store(output, at, value)?;
    }
    Ok(())
}
fn entry_store(
    bytes: &mut [u8],
    base: usize,
    index: usize,
    words: [u32; 3],
) -> Result<(), Diagnostic> {
    let start = base.checked_add(index.checked_mul(12).ok_or_else(budget)?).ok_or_else(budget)?;
    for (word, value) in words.into_iter().enumerate() {
        store(bytes, start + word * 4, value)?;
    }
    Ok(())
}
fn store(bytes: &mut [u8], at: usize, value: u32) -> Result<(), Diagnostic> {
    bytes
        .get_mut(at..at.checked_add(4).ok_or_else(budget)?)
        .ok_or_else(invariant)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
pub(super) fn category(kind: TypeCategory) -> u32 {
    match kind {
        TypeCategory::Bool => 0,
        TypeCategory::I32 => 1,
        TypeCategory::String => 2,
        TypeCategory::Struct => 3,
        TypeCategory::Enum => 4,
        TypeCategory::FixedArray => 5,
        TypeCategory::Vec => 6,
        TypeCategory::Shared => 7,
        TypeCategory::Weak => 8,
    }
}
pub(super) fn clone_kind(kind: CloneNodeKind) -> u32 {
    match kind {
        CloneNodeKind::Copy => 1,
        CloneNodeKind::String => 2,
        CloneNodeKind::Enum => 3,
        CloneNodeKind::Struct => 4,
        CloneNodeKind::FixedArray { .. } => 5,
        CloneNodeKind::VecEach => 6,
        CloneNodeKind::SharedCount => 7,
        CloneNodeKind::WeakCount => 8,
    }
}
pub(in crate::generic_execution_v1) use verify::check;
