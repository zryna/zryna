//! Storage-only recursion detection and the physical bump-allocator/frame injection bound.
use super::{
    Diagnostic, MEMORY_BYTES, RECURSIVE_FRAME_BYTES, RECURSIVE_MEMBER_BYTES,
    RECURSIVE_METADATA_HEADER_BYTES, RECURSIVE_TYPE_BYTES, add, align, budget, invariant, raw,
};
use crate::generic_execution_v1::work;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod graph;
#[cfg(test)]
mod tests;

// Both existing positive canonical allocation paths round align8(cursor+16+payload),
// payload>=1. The VM/audit must retain that kernel rule and unique live-owner frames.
const MIN_POSITIVE_ALLOCATION_BYTES: u32 = 24;

pub(super) fn required(
    program: &VerifiedExecution<'_>,
    spent: &mut usize,
) -> Result<bool, Diagnostic> {
    graph::required(program, spent)
}

pub(super) fn partition(
    program: &VerifiedExecution<'_>,
    source_end: u32,
    needed: bool,
    spent: &mut usize,
) -> Result<(Option<raw::RecursiveArena>, u32), Diagnostic> {
    let mut work = *spent;
    work::take(&mut work, 1)?;
    if source_end > MEMORY_BYTES || !source_end.is_multiple_of(8) {
        return Err(invariant());
    }
    let arena = if needed {
        let metadata_bytes = metadata(program, &mut work)?;
        let start = add(source_end, metadata_bytes)?;
        let remaining = MEMORY_BYTES.checked_sub(start).ok_or_else(budget)?;
        let (capacity, bytes) = split(remaining)?;
        Some(raw::RecursiveArena {
            metadata_start: source_end,
            metadata_bytes,
            start,
            bytes,
            capacity,
        })
    } else {
        None
    };
    let heap = match &arena {
        Some(arena) => add(arena.start, arena.bytes)?,
        None => source_end,
    };
    *spent = work;
    Ok((arena, heap))
}

fn split(remaining: u32) -> Result<(u32, u32), Diagnostic> {
    if remaining > MEMORY_BYTES {
        return Err(invariant());
    }
    let blocks = remaining / MIN_POSITIVE_ALLOCATION_BYTES;
    let capacity = blocks.checked_add(1).ok_or_else(budget)? / 2;
    let bytes = capacity.checked_mul(RECURSIVE_FRAME_BYTES).ok_or_else(budget)?;
    let heap = remaining.checked_sub(bytes).ok_or_else(budget)?;
    if heap / MIN_POSITIVE_ALLOCATION_BYTES > capacity {
        return Err(invariant());
    }
    Ok((capacity, bytes))
}

fn metadata(program: &VerifiedExecution<'_>, spent: &mut usize) -> Result<u32, Diagnostic> {
    let types = program.layouts(zryna_layout::StorageTarget::Linear32V1).types();
    let count = types.len();
    work::take(spent, count)?;
    let mut members = 0usize;
    for ty in types {
        work::take(spent, ty.fields().len().checked_add(ty.variants().len()).ok_or_else(budget)?)?;
        for _ in ty.fields() {
            members = members.checked_add(1).ok_or_else(budget)?;
        }
        for _ in ty.variants() {
            members = members.checked_add(1).ok_or_else(budget)?;
        }
    }
    metadata_bytes(count, members)
}
fn metadata_bytes(types: usize, members: usize) -> Result<u32, Diagnostic> {
    let total = types
        .checked_mul(RECURSIVE_TYPE_BYTES as usize)
        .and_then(|n| n.checked_add(members.checked_mul(RECURSIVE_MEMBER_BYTES as usize)?))
        .and_then(|n| n.checked_add(RECURSIVE_METADATA_HEADER_BYTES as usize))
        .ok_or_else(budget)?;
    align(u32::try_from(total).map_err(|_| budget())?, 8)
}

// Reject impossible/overlapping raw intervals before source inventory allocations.
// Independent derivation below still authenticates the claimed source frame extent
// and whether this exact program requires the arena; this is not a caller flag.
pub(super) fn geometry(
    claim: &raw::TargetMemoryPlan<'_, '_>,
    spent: &mut usize,
) -> Result<(), Diagnostic> {
    let mut work = *spent;
    work::take(&mut work, 1)?;
    let end = align(add(claim.frames_start, claim.frames_bytes)?, 8)?;
    match &claim.recursive {
        Some(arena) => {
            let bytes = arena.capacity.checked_mul(RECURSIVE_FRAME_BYTES).ok_or_else(budget)?;
            if arena.metadata_start != end
                || arena.metadata_bytes < RECURSIVE_METADATA_HEADER_BYTES
                || !arena.metadata_bytes.is_multiple_of(8)
                || arena.start != add(arena.metadata_start, arena.metadata_bytes)?
                || !arena.start.is_multiple_of(8)
                || arena.bytes != bytes
                || claim.heap_start != add(arena.start, arena.bytes)?
            {
                return Err(invariant());
            }
        }
        None if claim.heap_start != end => return Err(invariant()),
        None => {}
    }
    *spent = work;
    Ok(())
}
