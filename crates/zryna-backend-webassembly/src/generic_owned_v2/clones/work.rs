//! Prepay complete possible expansion before range retention, extra locals or encoding.
use super::{CloneNodeKind, Node, VerifiedStructClone, budget, invariant};
use crate::generic_owned_v2::MAX_BYTES;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram;
use zryna_layout::{StorageTarget, TypeCategory};

pub(in super::super) fn preflight(
    program: &VerifiedOwnedProgram<'_>,
    clone: VerifiedStructClone<'_>,
    spent: &mut usize,
) -> Result<(), Diagnostic> {
    let mut work = *spent;
    let strings = usize::try_from(clone.string_occurrences()).map_err(|_| budget())?;
    // Reject compact DAGs with excessive expanded String occurrences before unfolding them.
    charge(&mut work, strings.checked_mul(2048).ok_or_else(budget)?)?;
    u32::try_from(strings).map_err(|_| budget())?;
    if (strings != 0) != clone.failure_step().is_some() {
        return Err(invariant());
    }
    let root_width = count(program, clone.root(), &mut work, 0)?;
    // Root zeroing and site block/status/cursor templates, including independent audit visits.
    charge(&mut work, root_width.checked_mul(32).ok_or_else(budget)?)?;
    charge(&mut work, 256)?;
    *spent = work;
    Ok(())
}

fn count(
    program: &VerifiedOwnedProgram<'_>,
    node: Node<'_>,
    work: &mut usize,
    depth: usize,
) -> Result<usize, Diagnostic> {
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(invariant());
    }
    charge(work, 256)?;
    let lanes = width(program, node.ty(), work, 0)?;
    match node.kind() {
        CloneNodeKind::Copy => charge(work, lanes.checked_mul(32).ok_or_else(budget)?)?,
        CloneNodeKind::String => {
            if lanes != 3 {
                return Err(invariant());
            }
        }
        CloneNodeKind::Struct => {
            // Two checked range vectors (forward and rollback), iterator visits and offsets.
            charge(work, node.fields().len().checked_mul(64).ok_or_else(budget)?)?;
            for (_, _, child) in node.fields() {
                count(program, child, work, depth + 1)?;
            }
        }
        CloneNodeKind::Enum => {
            charge(work, node.variants().len().checked_mul(256).ok_or_else(budget)?)?;
            for (_, _, child) in node.variants() {
                if let Some(child) = child {
                    count(program, child, work, depth + 1)?;
                }
            }
        }
    }
    Ok(lanes)
}

fn width(
    program: &VerifiedOwnedProgram<'_>,
    index: u32,
    work: &mut usize,
    depth: usize,
) -> Result<usize, Diagnostic> {
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(invariant());
    }
    charge(work, usize::try_from(index).map_err(|_| budget())?.checked_add(8).ok_or_else(budget)?)?;
    let view = program
        .layouts(StorageTarget::Linear32V1)
        .types()
        .nth(index as usize)
        .ok_or_else(invariant)?;
    let lanes = match view.category() {
        TypeCategory::Bool | TypeCategory::I32 => 1,
        TypeCategory::String => 3,
        TypeCategory::Struct => {
            charge(work, view.fields().len().checked_mul(4).ok_or_else(budget)?)?;
            let mut lanes = 0usize;
            for (_, field, _) in view.fields() {
                lanes = lanes
                    .checked_add(width(program, field.index(), work, depth + 1)?)
                    .ok_or_else(budget)?;
            }
            lanes
        }
        TypeCategory::Enum => {
            charge(work, view.variants().len().checked_mul(4).ok_or_else(budget)?)?;
            let mut lanes = 0usize;
            for (_, child) in view.variants() {
                if let Some(child) = child {
                    lanes = lanes.max(width(program, child.index(), work, depth + 1)?);
                }
            }
            lanes.checked_add(1).ok_or_else(budget)?
        }
        _ => return Err(invariant()),
    };
    if lanes > crate::generic_owned_v2::MAX_TYPE_LANES as usize {
        return Err(budget());
    }
    Ok(lanes)
}

fn charge(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    *work = work.checked_add(count).filter(|n| *n <= MAX_BYTES).ok_or_else(budget)?;
    Ok(())
}
