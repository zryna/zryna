//! Independent actual SSA acquire proof.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, args, compare,
    constant, flow, invariant, load, origin, state, stored,
};
mod outcomes;
pub(in crate::generic_execution_v1::accounting) fn check(
    f: &Function,
    context: Value,
    call: Inst,
    merged: Value,
    operation: LogicalOperation,
    blocks: (Block, Block, Block),
) -> Result<(), Diagnostic> {
    let (rejected, admitted, merge) = blocks;
    origin(f, context)?;
    if f.layout.inst_block(call) != Some(admitted) {
        return Err(invariant());
    }
    let mut instructions = f.layout.block_insts(admitted);
    let a = stored(f, instructions.next().ok_or_else(invariant)?, context, 0)?;
    let s = stored(f, instructions.next().ok_or_else(invariant)?, context, 16)?;
    let h = stored(f, instructions.next().ok_or_else(invariant)?, context, 24)?;
    if instructions.next() != Some(call) {
        return Err(invariant());
    }
    let mut stores = 0;
    for inst in f.layout.block_insts(admitted) {
        if let D::Store { args, .. } = f.dfg.insts[inst]
            && args[1] == context
        {
            stores += 1;
        }
    }
    if stores != 5 {
        return Err(invariant());
    }
    let (attempt, old_a, old_s, old_h) = candidates(f, context, call, operation, (a, s, h))?;
    let live = flow::admission(f, context, (a, s, h), attempt, rejected, admitted)?;
    let guard = f.layout.prev_block(rejected).ok_or_else(invariant)?;
    let old_l = args(f, live, O::Iadd, 2)?[0];
    state::validation(f, context, guard, call, (old_a, old_l, old_s, old_h))?;
    state::zero(f, guard, *f.dfg.inst_args(call).last().ok_or_else(invariant)?)?;
    flow::results(
        f,
        context,
        call,
        merged,
        (rejected, admitted, merge),
        (live, h, old_a, old_s, old_h),
    )?;
    outcomes::check(f, call, context, operation)
}
fn candidates(
    f: &Function,
    context: Value,
    call: Inst,
    operation: LogicalOperation,
    values: (Value, Value, Value),
) -> Result<(Value, Value, Value, Value), Diagnostic> {
    let (a, s, h) = values;
    let aa = args(f, a, O::Iadd, 2)?;
    let ss = args(f, s, O::Iadd, 2)?;
    let hh = args(f, h, O::Iadd, 2)?;
    if !load(f, aa[0], context, 0)
        || !load(f, ss[0], context, 16)
        || !constant(f, ss[1], 1)
        || !load(f, hh[0], context, 24)
    {
        return Err(invariant());
    }
    let attempt = args(f, aa[1], O::Uextend, 1)?[0];
    let debt = args(f, hh[1], O::Uextend, 1)?[0];
    let both = args(f, attempt, O::Band, 2)?;
    if both[0] != debt {
        return Err(invariant());
    }
    let nonempty = compare(f, both[1], IntCC::NotEqual)?;
    let possible = compare(f, debt, IntCC::UnsignedLessThanOrEqual)?;
    if nonempty[0] != possible[0]
        || !constant(f, nonempty[1], 0)
        || !constant(f, possible[1], zryna_ownership_runtime_abi::MAX_STRING_BYTES.cast_signed())
    {
        return Err(invariant());
    }
    let operands = f.dfg.inst_args(call);
    match operation {
        LogicalOperation::StringFromUtf8Copy if operands.len() == 3 => {
            if possible[0] != operands[1] {
                return Err(invariant());
            }
        }
        LogicalOperation::StringClone if operands.len() == 2 => {
            if !load(f, possible[0], operands[0], 8) {
                return Err(invariant());
            }
        }
        LogicalOperation::StringConcat if operands.len() == 3 => {
            let sum = args(f, possible[0], O::Iadd, 2)?;
            if !load(f, sum[0], operands[0], 8) || !load(f, sum[1], operands[1], 8) {
                return Err(invariant());
            }
        }
        _ => return Err(invariant()),
    }
    Ok((aa[1], aa[0], ss[0], hh[0]))
}
