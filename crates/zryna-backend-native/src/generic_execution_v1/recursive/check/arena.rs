//! Independent complete lifecycle recipe: fresh address/count/owner reads, no other effects.
use super::super::calls::{Kind, Refs, signature};
use super::cursor::Cursor;
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{Function, Opcode as O, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
pub(super) fn check(f: &Function, r: Refs) -> Result<(), Diagnostic> {
    if f.signature != signature(Kind::Arena)
        || !f.sized_stack_slots.is_empty()
        || !f.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let block = f.layout.entry_block().ok_or_else(invariant)?;
    if f.layout.next_block(block).is_some() {
        return Err(invariant());
    }
    let parameters = f.dfg.block_params(block);
    if parameters.len() != 1 {
        return Err(invariant());
    }
    let ctx = parameters[0];
    let mut c = Cursor::new(f, block);
    let base = c.call(r.getter, &[ctx], types::I64)?;
    let used = c.load(types::I32, base, 0)?;
    let busy = c.load(types::I32, base, 4)?;
    let owner = c.load(types::I64, base, 8)?;
    let limit = c.constant(
        types::I32,
        i64::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| invariant())?,
    )?;
    let excess = c.compare(IntCC::UnsignedGreaterThan, [used, limit])?;
    let one = c.constant(types::I32, 1)?;
    let inactive = c.compare(IntCC::NotEqual, [busy, one])?;
    let foreign = c.compare(IntCC::NotEqual, [owner, ctx])?;
    let bad = c.binary(O::Bor, types::I8, [excess, inactive])?;
    let bad = c.binary(O::Bor, types::I8, [bad, foreign])?;
    c.trap(bad)?;
    c.returned(&[base])
}
