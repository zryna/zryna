//! Existing owning ceilings, with preallocation counters shared by decode and raw preflight.
use super::Failure;
use crate::data_ownership_v1 as m3;
pub(super) fn exceeded() -> Failure {
    crate::generic_v1::budget("full-profile wire inherited resource ceiling exceeded")
}
pub(super) fn bound(value: usize, limit: usize) -> Result<(), Failure> {
    if value > limit { Err(exceeded()) } else { Ok(()) }
}
pub(super) fn add(total: &mut usize, value: usize, limit: usize) -> Result<(), Failure> {
    let next = total.checked_add(value).ok_or_else(exceeded)?;
    bound(next, limit)?;
    *total = next;
    Ok(())
}
#[derive(Default)]
pub(super) struct Totals {
    parameters: usize,
    generic_functions: usize,
    blocks: usize,
    values: usize,
    edges: usize,
    calls: usize,
    aggregate: usize,
    literals: usize,
    function_values: usize,
    function_edges: usize,
    function_transitions: usize,
    function_drops: usize,
}
impl Totals {
    pub(super) fn generic_function(&mut self) -> Result<(), Failure> {
        add(&mut self.generic_functions, 1, 4096)
    }
    pub(super) fn function(&mut self) {
        self.function_values = 0;
        self.function_edges = 0;
        self.function_transitions = 0;
        self.function_drops = 0;
    }
    pub(super) fn parameters(&mut self, count: usize) -> Result<(), Failure> {
        bound(count, m3::MAX_PARAMETERS_PER_FUNCTION)?;
        add(&mut self.parameters, count, m3::MAX_PARAMETERS_PER_PROGRAM)
    }
    pub(super) fn blocks(&mut self, count: usize) -> Result<(), Failure> {
        bound(count, m3::MAX_BLOCKS_PER_FUNCTION)?;
        add(&mut self.blocks, count, m3::MAX_BLOCKS_PER_PROGRAM)
    }
    pub(super) fn values(&mut self, count: usize) -> Result<(), Failure> {
        add(&mut self.function_values, count, m3::MAX_VALUES_PER_FUNCTION)?;
        add(&mut self.values, count, m3::MAX_VALUES_PER_PROGRAM)
    }
    pub(super) fn edge(&mut self) -> Result<(), Failure> {
        self.edges(1)
    }
    pub(super) fn edges(&mut self, count: usize) -> Result<(), Failure> {
        add(&mut self.function_edges, count, m3::MAX_CFG_EDGES_PER_FUNCTION)?;
        add(&mut self.edges, count, m3::MAX_CFG_EDGES_PER_PROGRAM)
    }
    pub(super) fn call(&mut self) -> Result<(), Failure> {
        add(&mut self.calls, 1, m3::MAX_CALL_EDGES)
    }
    pub(super) fn aggregate(&mut self, count: usize) -> Result<(), Failure> {
        add(&mut self.aggregate, count, m3::MAX_AGGREGATE_OPERANDS)
    }
    pub(super) fn literal(&mut self, count: usize) -> Result<(), Failure> {
        bound(count, 64 * 1024)?;
        add(&mut self.literals, count, m3::MAX_STRING_LITERAL_BYTES)
    }
    pub(super) fn transitions(&mut self, count: usize) -> Result<(), Failure> {
        add(&mut self.function_transitions, count, m3::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION)
    }
    pub(super) fn drops(&mut self, count: usize) -> Result<(), Failure> {
        add(&mut self.function_drops, count, m3::MAX_DROP_ACTIONS_PER_FUNCTION)
    }
}
