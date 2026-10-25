//! CFG denial/result joins and held release checks, read from actual instructions.
use super::{
    A, Block, Function, Inst, L, O, S, Value, args, compare, constant, invariant, limit, load,
    stored,
};
use cranelift_codegen::ir::condcodes::IntCC;
use zryna_diagnostics::Diagnostic;
pub(super) fn admission(
    f: &Function,
    context: Value,
    candidates: (Value, Value, Value),
    attempt: Value,
    rejected: Block,
    admitted: Block,
) -> Result<Value, Diagnostic> {
    let (a, s, h) = candidates;
    let before = f.layout.prev_block(rejected).ok_or_else(invariant)?;
    let branch = f.layout.last_inst(before).ok_or_else(invariant)?;
    let targets =
        f.dfg.insts[branch].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
    if targets.len() != 2
        || targets[0].block(&f.dfg.value_lists) != rejected
        || targets[1].block(&f.dfg.value_lists) != admitted
    {
        return Err(invariant());
    }
    let cond = *f.dfg.inst_args(branch).first().ok_or_else(invariant)?;
    let pieces = args(f, cond, O::Bor, 2)?;
    let status = compare(f, pieces[0], IntCC::UnsignedGreaterThan)?;
    let sum = args(f, status[0], O::Iadd, 2)?;
    if sum != [s, h] || !constant(f, status[1], S.cast_signed()) {
        return Err(invariant());
    }
    let allocations = args(f, pieces[1], O::Bor, 2)?;
    limit(f, allocations[0], a, A)?;
    let live = compare(f, allocations[1], IntCC::UnsignedGreaterThan)?;
    if !constant(f, live[1], L.cast_signed()) {
        return Err(invariant());
    }
    let growth = args(f, live[0], O::Iadd, 2)?;
    if !load(f, growth[0], context, 8) || growth[1] != attempt {
        return Err(invariant());
    }
    Ok(live[0])
}
pub(super) fn results(
    f: &Function,
    context: Value,
    call: Inst,
    merged: Value,
    blocks: (Block, Block, Block),
    values: (Value, Value, Value, Value, Value),
) -> Result<(), Diagnostic> {
    let (rejected, admitted, merge) = blocks;
    let (live, held, _, _, old_h) = values;
    if f.dfg.block_params(merge) != [merged] {
        return Err(invariant());
    }
    let mut denied_insts = f.layout.block_insts(rejected);
    let cap_inst = denied_insts.next().ok_or_else(invariant)?;
    let denied = denied_insts.next().ok_or_else(invariant)?;
    if denied_insts.next().is_some() || f.dfg.insts[cap_inst].opcode() != O::Iconst {
        return Err(invariant());
    }
    let denied_targets =
        f.dfg.insts[denied].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
    if denied_targets.len() != 1 || denied_targets[0].block(&f.dfg.value_lists) != merge {
        return Err(invariant());
    }
    let cap = denied_targets[0]
        .args(&f.dfg.value_lists)
        .next()
        .and_then(|x| x.as_value())
        .ok_or_else(invariant)?;
    if !constant(f, cap, 2) {
        return Err(invariant());
    }
    let result = f.dfg.inst_results(call)[0];
    let jump = f.layout.last_inst(admitted).ok_or_else(invariant)?;
    let targets = f.dfg.insts[jump].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
    if targets.len() != 1
        || targets[0].block(&f.dfg.value_lists) != merge
        || targets[0].args(&f.dfg.value_lists).next().and_then(|x| x.as_value()) != Some(result)
    {
        return Err(invariant());
    }
    let mut insts = f.layout.block_insts(admitted).rev();
    insts.next();
    let new_h = stored(f, insts.next().ok_or_else(invariant)?, context, 24)?;
    let new_l = stored(f, insts.next().ok_or_else(invariant)?, context, 8)?;
    let l = args(f, new_l, O::Select, 3)?;
    let h = args(f, new_h, O::Select, 3)?;
    let ok = compare(f, l[0], IntCC::Equal)?;
    if ok[0] != result
        || !constant(f, ok[1], 0)
        || h[0] != l[0]
        || l[1] != live
        || h[1] != held
        || !load(f, l[2], context, 8)
        || h[2] != old_h
    {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn release(
    f: &Function,
    context: Value,
    block: Block,
    call: Inst,
    address: Value,
    old_h: Value,
    offset: i32,
) -> Result<(), Diagnostic> {
    let mut funded = false;
    let mut funded_amount = None;
    let mut after = false;
    let mut live = false;
    let mut status = false;
    for inst in f.layout.block_insts(block) {
        if inst == call {
            after = true;
            continue;
        }
        if !after && f.dfg.insts[inst].opcode() == O::Trapnz {
            let value = f.dfg.inst_args(inst)[0];
            if let Ok(guards) = args(f, value, O::Bor, 2)
                && let Ok(missing) = compare(f, guards[0], IntCC::Equal)
                && missing[0] == old_h
                && constant(f, missing[1], 0)
            {
                let absent = compare(f, guards[1], IntCC::UnsignedLessThan)?;
                let amount = args(f, absent[1], O::Uextend, 1)?;
                let nonempty = compare(f, amount[0], IntCC::NotEqual)?;
                funded = load(f, absent[0], context, 8)
                    && load(f, nonempty[0], address, offset)
                    && constant(f, nonempty[1], 0);
                funded_amount = Some(absent[1]);
            }
        }
        if after && f.dfg.insts[inst].opcode() == O::Trapnz {
            status |= f.dfg.inst_args(inst) == f.dfg.inst_results(call);
        }
        if after && let Ok(value) = stored(f, inst, context, 8) {
            let sub = args(f, value, O::Isub, 2)?;
            live = load(f, sub[0], context, 8) && Some(sub[1]) == funded_amount;
            break;
        }
    }
    if !funded || !live || !status {
        return Err(invariant());
    }
    Ok(())
}
