//! New-domain dense SSA and complete dominance; pure CFG algorithms carry no old seal.
use super::{Failure, budget, raw, reject, reserve};

#[derive(Debug)]
struct Definition {
    ty: raw::ValueType,
    block: usize,
    position: Option<usize>,
}
#[derive(Debug)]
pub(in crate::generic_v1::execution_v1) struct Graph {
    definitions: Vec<Definition>,
    enter: Vec<usize>,
    exit: Vec<usize>,
}

impl Graph {
    pub(super) fn build(
        f: &raw::Function,
        types: usize,
        spent: &mut usize,
    ) -> Result<Self, Failure> {
        let mut work = *spent;
        if f.blocks.is_empty() || f.blocks.len() > crate::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION
        {
            return Err(reject("execution CFG requires a bounded nonempty block inventory"));
        }
        budget::charge(
            &mut work,
            f.blocks
                .len()
                .checked_add(f.parameters.len())
                .ok_or_else(|| super::resource_failure("execution CFG census overflow"))?,
        )?;
        let mut definitions =
            f.parameters.iter().filter(|p| matches!(p, raw::Parameter::Value(_))).count();
        let mut edge_count = 0usize;
        let mut arguments = 0usize;
        for block in &f.blocks {
            budget::charge(&mut work, block.instructions.len())?;
            definitions = definitions
                .checked_add(block.value_parameters.len())
                .and_then(|n| {
                    n.checked_add(block.instructions.iter().filter(|i| i.result.is_some()).count())
                })
                .ok_or_else(|| super::resource_failure("execution definition census overflow"))?;
            let edges = edge_count_of(&block.terminator);
            budget::charge(&mut work, edges)?;
            arguments =
                arguments.checked_add(argument_count(&block.terminator)?).ok_or_else(|| {
                    super::resource_failure("execution edge argument census overflow")
                })?;
            edge_count = edge_count
                .checked_add(edges)
                .ok_or_else(|| super::resource_failure("execution edge census overflow"))?;
        }
        if definitions > crate::data_ownership_v1::MAX_VALUES_PER_FUNCTION
            || edge_count > crate::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION
        {
            return Err(super::resource_failure(
                "execution CFG exceeds inherited value/edge limits",
            ));
        }
        let b = f.blocks.len();
        let cost = b
            .checked_mul(b)
            .and_then(|n| n.checked_mul(edge_count.checked_add(b)?))
            .and_then(|n| n.checked_add(definitions.checked_mul(4)?))
            .and_then(|n| n.checked_add(edge_count.checked_mul(4)?))
            .and_then(|n| n.checked_add(arguments))
            .ok_or_else(|| super::resource_failure("execution dominance census overflow"))?;
        budget::charge(&mut work, cost)?;
        let mut graph =
            Self { definitions: reserve(definitions)?, enter: Vec::new(), exit: Vec::new() };
        graph.definitions.resize_with(definitions, || Definition {
            ty: raw::ValueType::Unit,
            block: usize::MAX,
            position: None,
        });
        for p in &f.parameters {
            if let raw::Parameter::Value(d) = p {
                graph.define(d, 0, None, types)?;
            }
        }
        let mut successors = reserve(b)?;
        let mut predecessors = reserve(b)?;
        predecessors.resize_with(b, Vec::new);
        graph.index_blocks(f, types, &mut successors, &mut predecessors)?;
        acyclic(&successors)?;
        if graph.definitions.iter().any(|d| d.block == usize::MAX) {
            return Err(reject("execution value ID inventory is not complete dense coverage"));
        }
        let idom = crate::control_flow_v1::immediate_dominators(&successors, &predecessors)
            .ok_or_else(|| reject("execution CFG contains unreachable or invalid blocks"))?;
        (graph.enter, graph.exit) = crate::control_flow_v1::dominator_intervals(&idom);
        graph.validate_edges(f)?;
        *spent = work;
        Ok(graph)
    }
    fn index_blocks(
        &mut self,
        f: &raw::Function,
        types: usize,
        successors: &mut Vec<Vec<usize>>,
        predecessors: &mut [Vec<usize>],
    ) -> Result<(), Failure> {
        let b = f.blocks.len();
        for (bi, block) in f.blocks.iter().enumerate() {
            if block.id as usize != bi || (bi == 0 && !block.value_parameters.is_empty()) {
                return Err(reject("execution block IDs/entry parameters are noncanonical"));
            }
            for d in &block.value_parameters {
                self.define(d, bi, None, types)?;
            }
            for (position, i) in block.instructions.iter().enumerate() {
                if let Some(d) = &i.result {
                    self.define(d, bi, Some(position), types)?;
                }
            }
            let mut next = reserve(edge_count_of(&block.terminator))?;
            for (edge, _) in edges(&block.terminator)? {
                let target = edge.target as usize;
                if target == 0 || target >= b {
                    return Err(reject("execution edge targets entry or a foreign block"));
                }
                next.push(target);
                predecessors[target].try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                predecessors[target].push(bi);
            }
            successors.push(next);
        }
        Ok(())
    }
    fn validate_edges(&self, f: &raw::Function) -> Result<(), Failure> {
        for (bi, block) in f.blocks.iter().enumerate() {
            for (edge, binding) in edges(&block.terminator)? {
                let target = &f.blocks[edge.target as usize].value_parameters;
                let skip = usize::from(binding.is_some());
                if binding.is_some_and(|d| target.first() != Some(d))
                    || target.len() != edge.arguments.len() + skip
                {
                    return Err(reject(
                        "execution edge differs from exact generated binding/parameter arity",
                    ));
                }
                for (id, d) in edge.arguments.iter().zip(&target[skip..]) {
                    if self.operand(*id, bi, block.instructions.len())? != d.ty {
                        return Err(reject("execution edge argument type differs from target"));
                    }
                }
            }
        }
        Ok(())
    }
    fn define(
        &mut self,
        d: &raw::ValueDefinition,
        block: usize,
        position: Option<usize>,
        types: usize,
    ) -> Result<(), Failure> {
        let slot = self
            .definitions
            .get_mut(d.id as usize)
            .ok_or_else(|| reject("execution value ID is outside dense census"))?;
        if slot.block != usize::MAX {
            return Err(reject("execution value ID is duplicated"));
        }
        if matches!(d.ty, raw::ValueType::Stored(i) if i as usize >= types) {
            return Err(reject("execution value has a foreign stored type"));
        }
        *slot = Definition { ty: d.ty, block, position };
        Ok(())
    }
    pub(super) fn operand(
        &self,
        id: u32,
        block: usize,
        position: usize,
    ) -> Result<raw::ValueType, Failure> {
        let d = self
            .definitions
            .get(id as usize)
            .ok_or_else(|| reject("execution operand has an unknown value ID"))?;
        let present = if d.block == block {
            d.position.is_none_or(|p| p < position)
        } else {
            let begin =
                self.enter.get(block).ok_or_else(|| reject("execution use block is foreign"))?;
            let end =
                self.exit.get(block).ok_or_else(|| reject("execution use block is foreign"))?;
            self.enter[d.block] <= *begin && *end <= self.exit[d.block]
        };
        if !present {
            return Err(reject("execution value does not dominate its use"));
        }
        Ok(d.ty)
    }
    pub(super) fn types(&self) -> impl ExactSizeIterator<Item = raw::ValueType> + '_ {
        self.definitions.iter().map(|d| d.ty)
    }
}

pub(crate) fn edge_count_of(t: &raw::Terminator) -> usize {
    match t {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => 0,
        raw::Terminator::Jump(_) => 1,
        raw::Terminator::Branch { .. } | raw::Terminator::WeakUpgradeBranch { .. } => 2,
        raw::Terminator::ClosedEnumMatch { arms, .. } => arms.len(),
    }
}
pub(super) fn edges(
    t: &raw::Terminator,
) -> Result<Vec<(&raw::Edge, Option<&raw::ValueDefinition>)>, Failure> {
    let mut result = reserve(edge_count_of(t))?;
    match t {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => {}
        raw::Terminator::Jump(e) => result.push((e, None)),
        raw::Terminator::Branch { yes, no, .. } => result.extend([(yes, None), (no, None)]),
        raw::Terminator::ClosedEnumMatch { arms, .. } => {
            for arm in arms {
                result.push((
                    &arm.edge,
                    match &arm.binding {
                        Some(raw::MatchBinding::Value(v)) => Some(v),
                        _ => None,
                    },
                ));
            }
        }
        raw::Terminator::WeakUpgradeBranch { success_binding, success, expired, .. } => {
            result.extend([(success, Some(success_binding)), (expired, None)]);
        }
    }
    Ok(result)
}

fn acyclic(successors: &[Vec<usize>]) -> Result<(), Failure> {
    let mut degree = reserve(successors.len())?;
    degree.resize(successors.len(), 0usize);
    for row in successors {
        for &next in row {
            degree[next] += 1;
        }
    }
    let mut queue = reserve(successors.len())?;
    queue.extend((0..successors.len()).filter(|id| degree[*id] == 0));
    let mut visited = 0;
    while let Some(id) = queue.pop() {
        visited += 1;
        for &next in &successors[id] {
            degree[next] -= 1;
            if degree[next] == 0 {
                queue.push(next);
            }
        }
    }
    if visited != successors.len() {
        return Err(reject("execution control-flow loop admission is not implemented yet"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn argument_count(t: &raw::Terminator) -> Result<usize, Failure> {
    let count = match t {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Some(0),
        raw::Terminator::Jump(e) => Some(e.arguments.len()),
        raw::Terminator::Branch { yes, no, .. } => {
            yes.arguments.len().checked_add(no.arguments.len())
        }
        raw::Terminator::ClosedEnumMatch { arms, .. } => {
            arms.iter().try_fold(0usize, |n, a| n.checked_add(a.edge.arguments.len()))
        }
        raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
            success.arguments.len().checked_add(expired.arguments.len())
        }
    };
    count.ok_or_else(|| super::resource_failure("execution edge argument census overflow"))
}
