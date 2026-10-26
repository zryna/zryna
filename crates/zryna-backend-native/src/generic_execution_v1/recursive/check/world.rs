//! Closed CFG coverage: every real block is matched exactly once, including loop headers.
use super::cursor::Cursor;
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{Block, Function, Type};
use std::collections::BTreeSet;
use zryna_diagnostics::Diagnostic;
pub(super) struct World<'a> {
    pub f: &'a Function,
    seen: BTreeSet<Block>,
}
impl<'a> World<'a> {
    pub fn new(f: &'a Function) -> Self {
        Self { f, seen: BTreeSet::new() }
    }
    pub fn at(&mut self, block: Block, parameters: &[Type]) -> Result<Cursor<'a>, Diagnostic> {
        if !self.seen.insert(block)
            || self.f.layout.first_inst(block).is_none()
            || self
                .f
                .dfg
                .block_params(block)
                .iter()
                .map(|v| self.f.dfg.value_type(*v))
                .ne(parameters.iter().copied())
        {
            return Err(invariant());
        }
        Ok(Cursor::new(self.f, block))
    }
    pub fn finish(&self) -> Result<(), Diagnostic> {
        if self.f.layout.blocks().any(|b| !self.seen.contains(&b)) {
            return Err(invariant());
        }
        Ok(())
    }
}
