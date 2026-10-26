//! Independent Vec effect classifier and counter transfer from actual pre-codegen SSA.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, args, compare,
    constant, data, flow, invariant, load, origin, release, state, stored, types,
};
mod classification;
mod outcomes;
type CandidateValues = (Value, Value, Value, Value, Value, Value, Value, Value);
pub(super) fn check(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
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
    let (next_attempts, next_spent, next_held, old_a, old_s, old_h, attempt_value, debt) =
        candidates(f, context, call, admitted)?;
    let attempt = args(f, attempt_value, O::Uextend, 1)?[0];
    let both = args(f, attempt, O::Band, 2)?;
    let operands = f.dfg.inst_args(call);
    let metadata = classification::metadata(p, f, operands)?;
    classification::check(f, operands, operation, metadata.stride(), both[0], both[1], debt)?;
    let temporary = flow::admission(
        f,
        context,
        (next_attempts, next_spent, next_held),
        attempt_value,
        rejected,
        admitted,
    )?;
    let guard = f.layout.prev_block(rejected).ok_or_else(invariant)?;
    let old_l = args(f, temporary, O::Iadd, 2)?[0];
    state::validation(f, context, guard, call, (old_a, old_l, old_s, old_h))?;
    state::zero(f, guard, *operands.last().ok_or_else(invariant)?)?;
    if operation == LogicalOperation::VecReserve {
        classification::funded(f, guard, context, operands[1], old_l, old_h)?;
    }
    let mut tail = f.layout.block_insts(admitted).rev();
    tail.next();
    tail.next();
    let committed_l = stored(f, tail.next().ok_or_else(invariant)?, context, 8)?;
    let choose = args(f, committed_l, O::Select, 3)?;
    let after = args(f, choose[1], O::Isub, 2)?;
    if after[0] != temporary {
        return Err(invariant());
    }
    classification::retired(f, operation, operands, both[1], after[1])?;
    outcomes::check(f, call, both[0])?;
    flow::results(
        f,
        context,
        call,
        merged,
        (rejected, admitted, merge),
        (choose[1], next_held, old_a, old_s, old_h),
    )
}
pub(super) fn storage(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &Function,
    context: Value,
    call: Inst,
) -> Result<(), Diagnostic> {
    let operands = f.dfg.inst_args(call);
    if operands.len() != 2 {
        return Err(invariant());
    }
    classification::metadata(p, f, operands)?;
    release::storage(f, context, call, operands[1])
}

fn candidates(
    f: &Function,
    context: Value,
    call: Inst,
    admitted: Block,
) -> Result<CandidateValues, Diagnostic> {
    let mut instructions = f.layout.block_insts(admitted);
    let a = stored(f, instructions.next().ok_or_else(invariant)?, context, 0)?;
    let s = stored(f, instructions.next().ok_or_else(invariant)?, context, 16)?;
    let h = stored(f, instructions.next().ok_or_else(invariant)?, context, 24)?;
    if instructions.next() != Some(call) {
        return Err(invariant());
    }
    if f.layout
        .block_insts(admitted)
        .filter(|i| {
            matches!(f.dfg.insts[*i],
        D::Store { args, .. } if args[1] == context)
        })
        .count()
        != 5
    {
        return Err(invariant());
    }
    if f.layout.block_insts(admitted).filter(|i| matches!(f.dfg.insts[*i], D::Store { .. })).count()
        != 5
    {
        return Err(invariant());
    }
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
    Ok((a, s, h, aa[0], ss[0], hh[0], aa[1], hh[1]))
}
