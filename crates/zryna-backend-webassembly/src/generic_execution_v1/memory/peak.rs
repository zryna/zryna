//! Source-call arena peak, with exact checked acyclic depth and no helper-recursion claim.
use super::{Diagnostic, add, budget, invariant, raw};
pub(super) fn fill(
    functions: &mut [raw::FunctionFrame],
    work: &mut usize,
) -> Result<(), Diagnostic> {
    crate::generic_execution_v1::work::retained::<u8>(work, functions.len())?;
    crate::generic_execution_v1::work::retained::<usize>(work, functions.len())?;
    let mut states = Vec::new();
    #[cfg(test)]
    crate::generic_execution_v1::work::reserve_seen(functions.len());
    states.try_reserve_exact(functions.len()).map_err(|_| budget())?;
    states.resize(functions.len(), 0u8);
    let mut heights = Vec::new();
    heights.try_reserve_exact(functions.len()).map_err(|_| budget())?;
    heights.resize(functions.len(), 0usize);
    for id in 0..functions.len() {
        visit(id, 0, functions, &mut states, &mut heights, work)?;
    }
    Ok(())
}
fn visit(
    id: usize,
    depth: usize,
    functions: &mut [raw::FunctionFrame],
    states: &mut [u8],
    heights: &mut [usize],
    work: &mut usize,
) -> Result<u32, Diagnostic> {
    crate::generic_execution_v1::work::take(work, 1)?;
    if depth > zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH || id >= functions.len() {
        return Err(budget());
    }
    if states[id] == 1 {
        return Err(invariant());
    }
    if states[id] == 2 {
        if depth
            .checked_add(heights[id])
            .is_none_or(|n| n > zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH)
        {
            return Err(budget());
        }
        return Ok(functions[id].peak);
    }
    states[id] = 1;
    let mut child_peak = 0;
    for index in 0..functions[id].calls.len() {
        let child = functions[id].calls[index] as usize;
        child_peak = child_peak.max(visit(child, depth + 1, functions, states, heights, work)?);
        heights[id] = heights[id].max(heights[child] + 1);
    }
    functions[id].peak = add(functions[id].bytes, child_peak)?;
    states[id] = 2;
    Ok(functions[id].peak)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_peak_prepays_state_arrays_before_nonempty_reserve() {
        let mut frames = vec![raw::FunctionFrame {
            bytes: 8,
            peak: 0,
            values: vec![],
            places: vec![],
            calls: vec![],
        }];
        let mut spent = crate::generic_execution_v1::MAX_BYTES;
        crate::generic_execution_v1::work::reset_seen();
        assert!(fill(&mut frames, &mut spent).is_err());
        assert_eq!(crate::generic_execution_v1::work::seen(), 0);
        let mut spent = 0;
        fill(&mut frames, &mut spent).expect("fixed genuine source/target control");
        assert_eq!(frames[0].peak, 8);
        assert_eq!(crate::generic_execution_v1::work::seen(), 1);
    }
}
