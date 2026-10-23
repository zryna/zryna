//! Exact instance/root lookup and transitive failure without assuming calls are infallible.
use super::{Failure, Function, budget, raw, reject, reserve, resource_failure};
#[cfg(test)]
mod depth_tests;

pub(super) fn target(
    p: &raw::Program,
    op: &raw::Operation,
    work: &mut usize,
) -> Result<Option<usize>, Failure> {
    match op {
        raw::Operation::ClosedGenericCall { instance, .. } => {
            budget::charge(work, p.functions.len())?;
            p.functions
                .iter()
                .enumerate()
                .filter(|(_, f)| f.key.first() == Some(&0x40))
                .nth(*instance as usize)
                .map(|(i, _)| Some(i))
                .ok_or_else(|| reject("execution call has a foreign dense instance ID"))
        }
        raw::Operation::SourceCall { module, function, .. } => {
            budget::charge(
                work,
                p.functions
                    .len()
                    .checked_mul(11)
                    .ok_or_else(|| resource_failure("execution call key lookup overflow"))?,
            )?;
            let mut key = [0u8; 9];
            key[0] = 0x41;
            key[1..5].copy_from_slice(&module.to_le_bytes());
            key[5..].copy_from_slice(&function.to_le_bytes());
            p.functions
                .iter()
                .position(|f| f.key.as_slice() == key)
                .map(Some)
                .ok_or_else(|| reject("execution source call root is absent"))
        }
        _ => Ok(None),
    }
}
pub(super) fn fallibility(
    p: &raw::Program,
    functions: &mut [Function],
    work: &mut usize,
) -> Result<(), Failure> {
    budget::charge(
        work,
        functions
            .len()
            .checked_mul(4)
            .ok_or_else(|| resource_failure("execution call state census overflow"))?,
    )?;
    let mut state = reserve(functions.len())?;
    state.resize(functions.len(), 0u8);
    let mut heights = reserve(functions.len())?;
    heights.resize(functions.len(), 0usize);
    for id in 0..functions.len() {
        // Inherited depth counts the root function; cached heights count child edges.
        visit(id, functions, &mut state, &mut heights, 1, work)?;
    }
    for (fi, f) in p.functions.iter().enumerate() {
        for (bi, block) in f.blocks.iter().enumerate() {
            for (ii, i) in block.instructions.iter().enumerate() {
                if let Some(target) = functions[fi].callees[bi][ii] {
                    let failure = match &i.operation {
                        raw::Operation::ClosedGenericCall { failure, .. }
                        | raw::Operation::SourceCall { failure, .. } => *failure,
                        _ => return Err(reject("call index binds a non-call operation")),
                    };
                    if failure.is_some() != functions[target].may_fail {
                        return Err(reject(
                            "call failure claim differs from independently derived callee behavior",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
fn visit(
    id: usize,
    functions: &mut [Function],
    state: &mut [u8],
    heights: &mut [usize],
    depth: usize,
    work: &mut usize,
) -> Result<bool, Failure> {
    budget::charge(work, 1)?;
    if depth > crate::data_ownership_v1::MAX_STATIC_CALL_DEPTH {
        return Err(resource_failure("execution call depth exceeds inherited limit"));
    }
    if state[id] == 2 {
        if depth
            .checked_add(heights[id])
            .is_none_or(|n| n > crate::data_ownership_v1::MAX_STATIC_CALL_DEPTH)
        {
            return Err(resource_failure("execution cached call depth exceeds inherited limit"));
        }
        return Ok(functions[id].may_fail);
    }
    if state[id] == 1 {
        return Err(reject("execution source call graph is cyclic"));
    }
    state[id] = 1;
    for bi in 0..functions[id].callees.len() {
        for ii in 0..functions[id].callees[bi].len() {
            if let Some(target) = functions[id].callees[bi][ii] {
                budget::charge(work, 1)?;
                let falls = visit(target, functions, state, heights, depth + 1, work)?;
                functions[id].may_fail |= falls;
                heights[id] = heights[id].max(
                    heights[target]
                        .checked_add(1)
                        .ok_or_else(|| resource_failure("execution call height overflow"))?,
                );
                if heights[id] > crate::data_ownership_v1::MAX_STATIC_CALL_DEPTH {
                    return Err(resource_failure(
                        "execution source call depth exceeds inherited limit",
                    ));
                }
            }
        }
    }
    state[id] = 2;
    Ok(functions[id].may_fail)
}
