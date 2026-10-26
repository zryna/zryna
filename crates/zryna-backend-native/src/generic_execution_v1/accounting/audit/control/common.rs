//! Closed context-store and actual preflight obligations.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, O, Value, args, compare, constant, invariant,
    load, stored, types,
};
pub(super) struct Counters {
    pub a: Value,
    pub s: Value,
    pub h: Value,
    pub old_a: Value,
    pub old_s: Value,
    pub old_h: Value,
    pub attempt: Value,
    pub debt: Value,
}
pub(super) fn candidates(f: &Function, context: Value, call: Inst) -> Result<Counters, Diagnostic> {
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let mut instructions = f.layout.block_insts(block);
    let a = stored(f, instructions.next().ok_or_else(invariant)?, context, 0)?;
    let s = stored(f, instructions.next().ok_or_else(invariant)?, context, 16)?;
    let h = stored(f, instructions.next().ok_or_else(invariant)?, context, 24)?;
    if instructions.next() != Some(call) {
        return Err(invariant());
    }
    let count =
        f.layout.block_insts(block).filter(|i| matches!(f.dfg.insts[*i], D::Store { .. })).count();
    if count != 5 {
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
    Ok(Counters { a, s, h, old_a: aa[0], old_s: ss[0], old_h: hh[0], attempt: aa[1], debt: hh[1] })
}
pub(super) fn guard(f: &Function, block: Block, output: Option<Value>) -> Result<(), Diagnostic> {
    let mut stores = 0;
    for inst in f.layout.block_insts(block) {
        match f.dfg.insts[inst] {
            D::Call { .. } => return Err(invariant()),
            D::Store { args, offset, .. } => {
                if Some(args[1]) != output
                    || i32::from(offset) != 0
                    || !constant(f, args[0], 0)
                    || f.dfg.value_type(args[0]) != types::I64
                {
                    return Err(invariant());
                }
                stores += 1;
            }
            _ => {}
        }
    }
    if stores != usize::from(output.is_some()) {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn funded(
    f: &Function,
    block: Block,
    context: Value,
    old_l: Value,
    old_h: Value,
) -> Result<(), Diagnostic> {
    let trap = f
        .layout
        .block_insts(block)
        .filter(|i| f.dfg.insts[*i].opcode() == O::Trapnz)
        .nth(2)
        .ok_or_else(invariant)?;
    let pieces = args(f, f.dfg.inst_args(trap)[0], O::Bor, 2)?;
    let h = compare(f, pieces[0], IntCC::Equal)?;
    let l = compare(f, pieces[1], IntCC::Equal)?;
    if h[0] != old_h
        || !constant(f, h[1], 0)
        || l[0] != old_l
        || !constant(f, l[1], 0)
        || !load(f, old_l, context, 8)
        || !load(f, old_h, context, 24)
    {
        return Err(invariant());
    }
    Ok(())
}
