//! Prepay each metadata pass before it runs, then the full future state footprint.
use super::{Failure, budget, raw, resource_failure, typed};
#[cfg(test)]
mod tests;

pub(super) fn precharge(
    f: &raw::Function,
    values: usize,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let transitions = census(f, &mut work)?;
    // One complete copy prepays reads and all retained slots: 4P+2V+2L,
    // doubled for source/destination work, plus eight reserves and a transition guard.
    // The extra 12P prepays canonical state walks and whole-subtree normalization.
    // Owner/loan copies must retain full P/L capacity, even when their live orders are short.
    let footprint = f
        .places
        .len()
        .checked_mul(20)
        .and_then(|n| n.checked_add(values.checked_mul(4)?))
        .and_then(|n| n.checked_add(f.loans.len().checked_mul(4)?))
        .and_then(|n| n.checked_add(12))
        .ok_or_else(|| resource_failure("execution state footprint overflow"))?;
    let work_count = footprint
        .checked_mul(transitions)
        .ok_or_else(|| resource_failure("execution state work overflow"))?;
    budget::charge(&mut work, work_count)?;
    *spent = work;
    Ok(())
}
fn census(f: &raw::Function, spent: &mut usize) -> Result<usize, Failure> {
    let mut work = *spent;
    let b = f.blocks.len();
    budget::charge(&mut work, b)?;
    let instructions = f.blocks.iter().try_fold(0usize, |n, b| {
        n.checked_add(b.instructions.len())
            .ok_or_else(|| resource_failure("execution state instruction census overflow"))
    })?;
    if instructions > crate::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
        return Err(resource_failure("execution state transitions exceed inherited limit"));
    }
    budget::charge(&mut work, instructions)?;
    budget::charge(&mut work, b)?;
    let edges = f.blocks.iter().try_fold(0usize, |n, b| {
        n.checked_add(typed::cfg::edge_count_of(&b.terminator))
            .ok_or_else(|| resource_failure("execution edge debit overflow"))
    })?;
    budget::charge(&mut work, edges)?;
    budget::charge(&mut work, b)?;
    let transitions = transition_work(f)?;
    *spent = work;
    Ok(transitions)
}

fn transition_work(f: &raw::Function) -> Result<usize, Failure> {
    let mut count =
        f.blocks
            .len()
            .checked_mul(4)
            .and_then(|n| n.checked_add(f.parameters.len()))
            .ok_or_else(|| resource_failure("execution state transition census overflow"))?;
    for block in &f.blocks {
        let edges = typed::cfg::edge_count_of(&block.terminator);
        count = count
            .checked_add(
                edges
                    .checked_mul(2)
                    .ok_or_else(|| resource_failure("execution state edge overflow"))?,
            )
            .ok_or_else(|| resource_failure("execution state transitions overflow"))?;
        let args = match &block.terminator {
            raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
            raw::Terminator::Jump(e) => e.arguments.len(),
            raw::Terminator::Branch { yes, no, .. } => yes
                .arguments
                .len()
                .checked_add(no.arguments.len())
                .ok_or_else(|| resource_failure("execution edge argument census overflow"))?,
            raw::Terminator::ClosedEnumMatch { arms, .. } => {
                arms.iter().try_fold(0usize, |n, a| {
                    n.checked_add(a.edge.arguments.len())
                        .ok_or_else(|| resource_failure("execution arm argument census overflow"))
                })?
            }
            raw::Terminator::WeakUpgradeBranch { success, expired, .. } => success
                .arguments
                .len()
                .checked_add(expired.arguments.len())
                .ok_or_else(|| resource_failure("execution edge argument census overflow"))?,
        };
        count = count
            .checked_add(args)
            .ok_or_else(|| resource_failure("execution edge argument work overflow"))?;
        for i in &block.instructions {
            let fanout = match &i.operation {
                raw::Operation::StructConstruct { fields, .. } => fields.len(),
                raw::Operation::FixedArrayConstruct { elements, .. }
                | raw::Operation::VecConstruct { elements, .. } => elements.len(),
                raw::Operation::ClosedGenericCall { arguments, .. }
                | raw::Operation::SourceCall { arguments, .. } => arguments.len(),
                _ => 0,
            };
            count = count
                .checked_add(fanout)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| resource_failure("execution transfer fanout overflow"))?;
        }
    }
    Ok(count)
}
