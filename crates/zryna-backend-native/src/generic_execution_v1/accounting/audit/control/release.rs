//! Mandatory control cleanup is funded, never a source quota branch.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, acquire, args, common,
    compare, constant, invariant, load, origin, state, stored, types,
};
mod predicates;
pub(in crate::generic_execution_v1::accounting::audit) fn check(
    f: &Function,
    context: Value,
    call: Inst,
    operation: LogicalOperation,
) -> Result<(), Diagnostic> {
    origin(f, context)?;
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let operands = f.dfg.inst_args(call);
    let output = if operation == LogicalOperation::StrongReleaseFinish {
        if operands.len() != 1 {
            return Err(invariant());
        }
        None
    } else {
        if operands.len() != 2 {
            return Err(invariant());
        }
        Some(operands[1])
    };
    let hs = f.layout.prev_inst(call).ok_or_else(invariant)?;
    let ss = f.layout.prev_inst(hs).ok_or_else(invariant)?;
    let held = stored(f, hs, context, 24)?;
    let spent = stored(f, ss, context, 16)?;
    let h = args(f, held, O::Iadd, 2)?;
    let s = args(f, spent, O::Iadd, 2)?;
    if !load(f, h[0], context, 24)
        || !constant(f, h[1], -1)
        || !load(f, s[0], context, 16)
        || !constant(f, s[1], 1)
    {
        return Err(invariant());
    }
    let old_l = state::find_load(f, block, context, 8)?;
    state::validation(
        f,
        context,
        block,
        call,
        (state::find_load(f, block, context, 0)?, old_l, s[0], h[0]),
    )?;
    common::funded(f, block, context, old_l, h[0])?;
    stores(f, block, context, output, call)?;
    let amount = committed(f, block, context, call, old_l)?;
    predicates::check(f, block, call, operation, amount, output)
}
fn stores(
    f: &Function,
    block: Block,
    context: Value,
    output: Option<Value>,
    call: Inst,
) -> Result<(), Diagnostic> {
    let mut ctx = 0;
    let mut out = 0;
    let mut after = false;
    for inst in f.layout.block_insts(block) {
        if inst == call {
            after = true;
        }
        if let D::Store { args, offset, .. } = f.dfg.insts[inst] {
            if args[1] == context {
                ctx += 1;
            } else if Some(args[1]) == output
                && i32::from(offset) == 0
                && constant(f, args[0], 0)
                && f.dfg.value_type(args[0]) == types::I32
                && !after
            {
                out += 1;
            } else {
                return Err(invariant());
            }
        }
    }
    if ctx != 3 || out != usize::from(output.is_some()) {
        return Err(invariant());
    }
    Ok(())
}
fn committed(
    f: &Function,
    block: Block,
    context: Value,
    call: Inst,
    old_l: Value,
) -> Result<Value, Diagnostic> {
    let mut after = false;
    let mut status = false;
    let mut amount = None;
    for inst in f.layout.block_insts(block) {
        if let D::Call { .. } = f.dfg.insts[inst] {
            if inst != call {
                return Err(invariant());
            }
            after = true;
            continue;
        }
        if !after {
            continue;
        }
        if f.dfg.insts[inst].opcode() == O::Trapnz
            && f.dfg.inst_args(inst) == f.dfg.inst_results(call)
        {
            status = true;
        }
        if let Ok(live) = stored(f, inst, context, 8) {
            if !status {
                return Err(invariant());
            }
            let sub = args(f, live, O::Isub, 2)?;
            if sub[0] != old_l || amount.is_some() {
                return Err(invariant());
            }
            amount = Some(sub[1]);
        }
        if matches!(f.dfg.insts[inst],D::Load{arg,..} if Some(&arg)==f.dfg.inst_args(call).first())
        {
            return Err(invariant());
        }
    }
    amount.ok_or_else(invariant)
}
