//! Independent actual SSA release proof.
use super::{
    D, Diagnostic, Function, Inst, O, Value, args, constant, flow, invariant, load, origin, state,
    stored,
};
pub(in crate::generic_execution_v1::accounting) fn check(
    f: &Function,
    context: Value,
    call: Inst,
    address: Value,
) -> Result<(), Diagnostic> {
    check_offset(f, context, call, address, 8)
}
pub(super) fn storage(
    f: &Function,
    context: Value,
    call: Inst,
    address: Value,
) -> Result<(), Diagnostic> {
    check_offset(f, context, call, address, 16)
}
fn check_offset(
    f: &Function,
    context: Value,
    call: Inst,
    address: Value,
    offset: i32,
) -> Result<(), Diagnostic> {
    origin(f, context)?;
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let mut previous = None;
    let mut before = None;
    for inst in f.layout.block_insts(block) {
        if inst == call {
            break;
        }
        before = previous;
        previous = Some(inst);
    }
    let h = stored(f, previous.ok_or_else(invariant)?, context, 24)?;
    let s = stored(f, before.ok_or_else(invariant)?, context, 16)?;
    let hh = args(f, h, O::Iadd, 2)?;
    let ss = args(f, s, O::Iadd, 2)?;
    if !load(f, hh[0], context, 24)
        || !constant(f, hh[1], -1)
        || !load(f, ss[0], context, 16)
        || !constant(f, ss[1], 1)
    {
        return Err(invariant());
    }
    state::validation(
        f,
        context,
        block,
        call,
        (
            state::find_load(f, block, context, 0)?,
            state::find_load(f, block, context, 8)?,
            ss[0],
            hh[0],
        ),
    )?;
    let mut stores = 0;
    for inst in f.layout.block_insts(block) {
        if let D::Store { args, .. } = f.dfg.insts[inst]
            && args[1] == context
        {
            stores += 1;
        }
    }
    if stores != 3 {
        return Err(invariant());
    }
    flow::release(f, context, block, call, address, hh[0], offset)
}
