//! Existing pure topology algorithm with a separate paid loop-analysis inventory.
use super::{Failure, budget, reserve};
pub(super) fn check(
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
    enter: &[usize],
    exit: &[usize],
    edges: usize,
    spent: &mut usize,
) -> Result<Vec<bool>, Failure> {
    let blocks = successors.len();
    let classification = blocks
        .checked_mul(3)
        .and_then(|n| n.checked_add(edges))
        .ok_or_else(|| super::super::resource_failure("execution loop classification overflow"))?;
    budget::charge(spent, classification)?;
    let mut headers = reserve(blocks)?;
    headers.resize(blocks, false);
    let mut backedges = 0usize;
    for (source, targets) in successors.iter().enumerate() {
        for &target in targets {
            if enter[target] <= enter[source] && exit[source] <= exit[target] {
                headers[target] = true;
                backedges = backedges.checked_add(1).ok_or(Failure::InternalFailure)?;
            }
        }
    }
    let count = headers.iter().filter(|header| **header).count();
    let cost = traversal_cost(blocks, edges, count, backedges)?;
    budget::charge(spent, cost)?;
    crate::generic_v1::loops::check(successors, predecessors, enter, exit)?;
    Ok(headers)
}
fn traversal_cost(b: usize, e: usize, h: usize, k: usize) -> Result<usize, Failure> {
    // 16B: fixed arenas/initialization/header/queue/depth passes. 8E: forward
    // reservations, amortized copies, degree updates and DAG visits. 8HB:
    // membership capacity/initialization, pending nodes and nesting scans.
    // KB: each latch's reserved B-slot pending arena. 4HE: reverse predecessor
    // visits, comparisons and membership updates, once per distinct header.
    b.checked_mul(16)
        .and_then(|n| n.checked_add(e.checked_mul(8)?))
        .and_then(|n| n.checked_add(h.checked_mul(b)?.checked_mul(8)?))
        .and_then(|n| n.checked_add(k.checked_mul(b)?))
        .and_then(|n| n.checked_add(h.checked_mul(e)?.checked_mul(4)?))
        .and_then(|n| n.checked_add(8))
        .ok_or_else(|| super::super::resource_failure("execution natural-loop work overflow"))
}
