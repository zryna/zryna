//! Retain each opaque same-program Clone node once; indirection cycles are finite edges.
use super::{Diagnostic, budget, invariant, reserve, take};
use zryna_ir::generic_v1::execution_v1::{VerifiedCloneNode, VerifiedExecution};
pub(super) fn derive<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    count: usize,
    work: &mut u64,
) -> Result<Vec<Option<VerifiedCloneNode<'p, 'a>>>, Diagnostic> {
    take(
        work,
        (program.operations().len() as u64)
            .checked_mul(128)
            .and_then(|n| n.checked_add((count as u64).checked_mul(128)?))
            .ok_or_else(budget)?,
    )?;
    let mut nodes = reserve(count)?;
    nodes.resize(count, None);
    let mut queue = reserve(count)?;
    for op in program.operations() {
        if let Some(clone) = op.clone_frontier() {
            add(clone.frontier(), &mut nodes, &mut queue)?;
        }
    }
    let mut cursor = 0;
    while cursor < queue.len() {
        let node = queue[cursor];
        cursor += 1;
        take(
            work,
            (node.fields().len() as u64)
                .checked_add(node.variants().len() as u64)
                .and_then(|n| n.checked_add(1))
                .and_then(|n| n.checked_mul(128))
                .ok_or_else(budget)?,
        )?;
        for (_, child) in node.fields() {
            add(child, &mut nodes, &mut queue)?;
        }
        for (_, child) in node.variants() {
            if let Some(child) = child {
                add(child, &mut nodes, &mut queue)?;
            }
        }
        if let Some(child) = node.element() {
            add(child, &mut nodes, &mut queue)?;
        }
    }
    Ok(nodes)
}
fn add<'p, 'a>(
    node: VerifiedCloneNode<'p, 'a>,
    nodes: &mut [Option<VerifiedCloneNode<'p, 'a>>],
    queue: &mut Vec<VerifiedCloneNode<'p, 'a>>,
) -> Result<(), Diagnostic> {
    let row = nodes.get_mut(node.ty() as usize).ok_or_else(invariant)?;
    if row.is_none() {
        *row = Some(node);
        queue.push(node);
    }
    Ok(())
}
