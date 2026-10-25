//! Source-rooted ownership cycles, distinct from mere presence of recursive layouts.
use super::{
    Diagnostic, budget, invariant,
    planning::{reserve, take},
    shapes::Shape,
};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod clone_prepay;
mod graph;
mod prepay;
pub(super) use clone_prepay::emission as clone_emission;
mod roots;
pub(super) use prepay::emission;

pub(super) fn derive(
    program: &VerifiedExecution<'_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<bool, Diagnostic> {
    let graph = graph::Graph::derive(types, work)?;
    let mut reachable = reserve(types.len())?;
    reachable.resize(types.len(), false);
    let mut queue = reserve(types.len())?;
    roots::collect(program, &mut reachable, &mut queue, work)?;
    let mut cursor = 0;
    while cursor < queue.len() {
        let id = queue[cursor] as usize;
        cursor += 1;
        for &child in graph.children(id)? {
            let slot = reachable.get_mut(child as usize).ok_or_else(invariant)?;
            if !*slot {
                *slot = true;
                queue.push(child);
            }
        }
    }
    let total = queue.len();
    let mut incoming = reserve(types.len())?;
    incoming.resize(types.len(), 0u32);
    for &id in &queue {
        for &child in graph.children(id as usize)? {
            let slot = incoming.get_mut(child as usize).ok_or_else(invariant)?;
            *slot = slot.checked_add(1).ok_or_else(budget)?;
        }
    }
    queue.clear();
    for (id, (&live, &count)) in reachable.iter().zip(&incoming).enumerate() {
        if live && count == 0 {
            queue.push(u32::try_from(id).map_err(|_| budget())?);
        }
    }
    cursor = 0;
    while cursor < queue.len() {
        let id = queue[cursor] as usize;
        cursor += 1;
        for &child in graph.children(id)? {
            let slot = incoming.get_mut(child as usize).ok_or_else(invariant)?;
            *slot = slot.checked_sub(1).ok_or_else(invariant)?;
            if *slot == 0 {
                queue.push(child);
            }
        }
    }
    Ok(queue.len() != total)
}

#[cfg(test)]
mod tests;
