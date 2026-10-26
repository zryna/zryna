//! Each reachable source body is inspected once; static calls keep their inherited depth bound.
use super::super::{Context, Diagnostic, Function, cfg, charge, emission_budget, filled, internal};

pub(super) struct Graph {
    pub(super) functions: Vec<usize>,
    ranges: Vec<(usize, usize)>,
    calls: Vec<usize>,
}
impl Graph {
    pub(super) fn build(
        context: &Context<'_, '_>,
        entry: usize,
    ) -> Result<(Self, bool), Diagnostic> {
        let count = context.functions.len();
        let mut seen = filled(context, count, false)?;
        let mut functions = filled(context, count, 0usize)?;
        functions.clear();
        let mut ranges = filled(context, count, (0usize, 0usize))?;
        let mut calls = filled(context, context.program.operations().len(), 0usize)?;
        calls.clear();
        *seen.get_mut(entry).ok_or_else(internal)? = true;
        functions.push(entry);
        let mut next = 0;
        let mut cyclic = false;
        while next < functions.len() {
            charge(&context.work, 1)?;
            let index = functions[next];
            next += 1;
            let function = context.functions.get(index).ok_or_else(internal)?;
            cyclic |= has_cycle(context, function)?;
            let start = calls.len();
            charge(&context.work, function.blocks.len())?;
            for block in &function.blocks {
                let operations = block.operations();
                charge(&context.work, operations.len())?;
                for operation in operations {
                    if matches!(
                        operation.kind(),
                        super::raw::Operation::SourceCall { .. }
                            | super::raw::Operation::ClosedGenericCall { .. }
                    ) {
                        let target = operation.call_target().ok_or_else(internal)?.index();
                        calls.push(target);
                        let known = seen.get_mut(target).ok_or_else(internal)?;
                        if !*known {
                            *known = true;
                            functions.push(target);
                        }
                    }
                }
            }
            ranges[index] = (start, calls.len());
        }
        Ok((Self { functions, ranges, calls }, cyclic))
    }
    pub(super) fn check_depth(
        &self,
        context: &Context<'_, '_>,
        entry: usize,
    ) -> Result<(), Diagnostic> {
        let count = context.functions.len();
        let mut degree = filled(context, count, 0usize)?;
        let mut height = filled(context, count, 0usize)?;
        let mut queue = filled(context, count, 0usize)?;
        queue.clear();
        charge(
            &context.work,
            self.calls
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(self.functions.len().checked_mul(2)?))
                .ok_or_else(emission_budget)?,
        )?;
        for &target in &self.calls {
            degree[target] = degree[target].checked_add(1).ok_or_else(emission_budget)?;
        }
        for &function in &self.functions {
            if degree[function] == 0 {
                queue.push(function);
            }
        }
        height[entry] = 1;
        let mut visited = 0usize;
        while let Some(function) = queue.pop() {
            visited += 1;
            let (start, end) = self.ranges[function];
            for &target in &self.calls[start..end] {
                let depth = height[function].checked_add(1).ok_or_else(emission_budget)?;
                if depth > zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH {
                    return Err(emission_budget());
                }
                height[target] = height[target].max(depth);
                degree[target] -= 1;
                if degree[target] == 0 {
                    queue.push(target);
                }
            }
        }
        if visited != self.functions.len() {
            return Err(emission_budget());
        }
        Ok(())
    }
}
fn has_cycle(context: &Context<'_, '_>, function: &Function<'_, '_>) -> Result<bool, Diagnostic> {
    let count = function.blocks.len();
    let mut degree = filled(context, count, 0usize)?;
    let mut queue = filled(context, count, 0usize)?;
    queue.clear();
    charge(&context.work, count)?;
    let edges = function.blocks.iter().try_fold(0usize, |total, block| {
        let count = match block.terminator() {
            super::raw::Terminator::Jump(_) => 1,
            super::raw::Terminator::Branch { .. }
            | super::raw::Terminator::WeakUpgradeBranch { .. } => 2,
            super::raw::Terminator::ClosedEnumMatch { arms, .. } => arms.len(),
            super::raw::Terminator::Return { .. } | super::raw::Terminator::Trap { .. } => 0,
        };
        total.checked_add(count).ok_or_else(emission_budget)
    })?;
    charge(
        &context.work,
        edges
            .checked_mul(2)
            .and_then(|n| n.checked_add(count.checked_mul(3)?))
            .ok_or_else(emission_budget)?,
    )?;
    for block in &function.blocks {
        for (edge, _) in cfg::edges(block.terminator()) {
            degree[edge.target as usize] =
                degree[edge.target as usize].checked_add(1).ok_or_else(emission_budget)?;
        }
    }
    for (index, &incoming) in degree.iter().enumerate() {
        if incoming == 0 {
            queue.push(index);
        }
    }
    let mut visited = 0usize;
    while let Some(index) = queue.pop() {
        visited += 1;
        for (edge, _) in cfg::edges(function.blocks[index].terminator()) {
            let incoming = &mut degree[edge.target as usize];
            *incoming -= 1;
            if *incoming == 0 {
                queue.push(edge.target as usize);
            }
        }
    }
    Ok(visited != count)
}
