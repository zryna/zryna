//! Kahn traversal of owned edges reachable from actual stored or certified Clone roots.
use super::{Diagnostic, VerifiedExecution, budget, invariant, work};
use zryna_layout::{StorageTarget, TypeCategory, generic_v1::TypeView};
mod roots;

pub(super) fn required(
    program: &VerifiedExecution<'_>,
    spent: &mut usize,
) -> Result<bool, Diagnostic> {
    let mut work = *spent;
    let layout = program.layouts(StorageTarget::Linear32V1);
    let count = layout.types().len();
    work::retained::<TypeView<'_>>(&mut work, count)?;
    work::take(&mut work, count)?;
    let mut types = Vec::new();
    reserve(&mut types, count)?;
    types.extend(layout.types());
    let reachable = reachable(program, &types, &mut work)?;
    work::take(&mut work, count)?;
    let active = reachable.iter().filter(|v| **v).count();
    if active == 0 {
        *spent = work;
        return Ok(false);
    }
    work::take(&mut work, count)?;
    let mut edge_count = 0usize;
    for (at, live) in reachable.iter().enumerate() {
        if *live {
            children(&types, at, &mut work, |_| {
                edge_count = edge_count.checked_add(1).ok_or_else(budget)?;
                Ok(())
            })?;
        }
    }
    let cyclic = cyclic(&types, &reachable, active, edge_count, &mut work)?;
    *spent = work;
    Ok(cyclic)
}

fn reachable(
    program: &VerifiedExecution<'_>,
    types: &[TypeView<'_>],
    spent: &mut usize,
) -> Result<Vec<bool>, Diagnostic> {
    let count = types.len();
    work::retained::<bool>(spent, count)?;
    work::retained::<u32>(spent, count)?;
    work::take(spent, count)?;
    let mut live = Vec::new();
    reserve(&mut live, count)?;
    live.resize(count, false);
    let mut queue = Vec::new();
    reserve(&mut queue, count)?;
    roots::seed(program, types, &mut live, &mut queue, spent)?;
    let mut done = 0usize;
    while done < queue.len() {
        work::take(spent, 1)?;
        let at = queue[done] as usize;
        children(types, at, spent, |id| roots::mark(id, types, &mut live, &mut queue))?;
        done += 1;
    }
    Ok(live)
}

fn cyclic(
    types: &[TypeView<'_>],
    live: &[bool],
    active: usize,
    edge_count: usize,
    spent: &mut usize,
) -> Result<bool, Diagnostic> {
    let count = types.len();
    let offsets_count = count.checked_add(1).ok_or_else(budget)?;
    work::retained::<u32>(spent, edge_count)?;
    work::retained::<usize>(spent, count)?;
    work::retained::<usize>(spent, offsets_count)?;
    work::retained::<u32>(spent, count)?;
    work::take(spent, offsets_count)?;
    let mut edges = Vec::new();
    reserve(&mut edges, edge_count)?;
    let mut incoming = Vec::new();
    reserve(&mut incoming, count)?;
    incoming.resize(count, 0usize);
    let mut offsets = Vec::new();
    reserve(&mut offsets, offsets_count)?;
    offsets.push(0);
    let mut queue = Vec::new();
    reserve(&mut queue, count)?;
    work::take(spent, count.checked_mul(2).ok_or_else(budget)?)?;
    for (at, enabled) in live.iter().enumerate() {
        if *enabled {
            children(types, at, spent, |id| {
                if !live.get(id as usize).copied().ok_or_else(invariant)? {
                    return Err(invariant());
                }
                let slot = incoming.get_mut(id as usize).ok_or_else(invariant)?;
                *slot = slot.checked_add(1).ok_or_else(budget)?;
                if edges.len() >= edge_count {
                    return Err(invariant());
                }
                edges.push(id);
                Ok(())
            })?;
        }
        offsets.push(edges.len());
    }
    if edges.len() != edge_count {
        return Err(invariant());
    }
    work::take(spent, count.checked_mul(3).ok_or_else(budget)?)?;
    for (at, indegree) in incoming.iter().enumerate() {
        if live[at] && *indegree == 0 {
            queue.push(u32::try_from(at).map_err(|_| budget())?);
        }
    }
    let mut done = 0usize;
    while done < queue.len() {
        work::take(spent, 3)?;
        let at = queue[done] as usize;
        let row = edges.get(offsets[at]..offsets[at + 1]).ok_or_else(invariant)?;
        work::take(spent, row.len().checked_mul(3).ok_or_else(budget)?)?;
        for id in row {
            let slot = incoming.get_mut(*id as usize).ok_or_else(invariant)?;
            *slot = slot.checked_sub(1).ok_or_else(invariant)?;
            if *slot == 0 {
                work::take(spent, 1)?;
                if queue.len() >= count {
                    return Err(invariant());
                }
                queue.push(*id);
            }
        }
        done += 1;
    }
    Ok(done != active)
}

fn reserve<T>(out: &mut Vec<T>, count: usize) -> Result<(), Diagnostic> {
    #[cfg(test)]
    work::reserve_seen(count);
    out.try_reserve_exact(count).map_err(|_| budget())
}

pub(super) fn children(
    types: &[TypeView<'_>],
    at: usize,
    spent: &mut usize,
    mut visit: impl FnMut(u32) -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    work::take(spent, 1)?;
    let ty = types.get(at).copied().ok_or_else(invariant)?;
    if ty.drop_kind() == 0 {
        return Ok(());
    }
    // One member's prepaid work covers its source row, target lookup, and
    // optional reachability/indegree/queue writes in the caller, before any visit.
    let mut child = |id: u32| -> Result<(), Diagnostic> {
        let target = types.get(id as usize).ok_or_else(invariant)?;
        if target.drop_kind() != 0 {
            visit(id)?;
        }
        Ok(())
    };
    match ty.category() {
        TypeCategory::Struct => {
            work::take(spent, ty.fields().len().checked_mul(6).ok_or_else(budget)?)?;
            for (_, id, _) in ty.fields() {
                child(id.index())?;
            }
        }
        TypeCategory::Enum => {
            work::take(spent, ty.variants().len().checked_mul(6).ok_or_else(budget)?)?;
            for (_, id) in ty.variants() {
                if let Some(id) = id {
                    child(id.index())?;
                }
            }
        }
        TypeCategory::FixedArray if ty.array_length() == Some(0) => {}
        TypeCategory::FixedArray | TypeCategory::Vec | TypeCategory::Shared => {
            work::take(spent, 6)?;
            child(ty.referenced_type().ok_or_else(invariant)?.index())?;
        }
        TypeCategory::Weak | TypeCategory::String | TypeCategory::Bool | TypeCategory::I32 => {}
    }
    Ok(())
}

// Compares a real source-bound layout universe with actual storage reachability.
// This test-only boolean graph control issues no source, address or memory authority.
#[cfg(test)]
pub(super) fn all_layouts_have_cycle(
    program: &VerifiedExecution<'_>,
    spent: &mut usize,
) -> Result<bool, Diagnostic> {
    let mut work = *spent;
    let layout = program.layouts(StorageTarget::Linear32V1);
    let count = layout.types().len();
    work::retained::<TypeView<'_>>(&mut work, count)?;
    work::take(&mut work, count)?;
    let mut types = Vec::new();
    reserve(&mut types, count)?;
    types.extend(layout.types());
    work::retained::<bool>(&mut work, count)?;
    work::take(&mut work, count)?;
    let mut live = Vec::new();
    reserve(&mut live, count)?;
    live.resize(count, true);
    work::take(&mut work, count)?;
    let mut edges = 0usize;
    for at in 0..count {
        children(&types, at, &mut work, |_| {
            edges = edges.checked_add(1).ok_or_else(budget)?;
            Ok(())
        })?;
    }
    let result = cyclic(&types, &live, count, edges, &mut work)?;
    *spent = work;
    Ok(result)
}
