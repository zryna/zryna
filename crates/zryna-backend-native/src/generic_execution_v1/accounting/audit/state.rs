//! Counter validity and zero outputs are independently read before the admission branch.
use super::{
    A, Block, D, Function, Inst, O, S, Value, args, compare, constant, invariant, limit, load,
};
use cranelift_codegen::ir::condcodes::IntCC;
use zryna_diagnostics::Diagnostic;
pub(super) fn validation(
    f: &Function,
    context: Value,
    block: Block,
    before: Inst,
    values: (Value, Value, Value, Value),
) -> Result<(), Diagnostic> {
    let (attempts, live, spent, held) = values;
    // A dominating earlier event's load has the same address but stale authority.
    // All four checked counter snapshots must originate inside this closed preflight.
    for value in [attempts, live, spent, held] {
        let inst = f.dfg.value_def(value).inst().ok_or_else(invariant)?;
        if f.layout.inst_block(inst) != Some(block) {
            return Err(invariant());
        }
    }
    let mut traps = f
        .layout
        .block_insts(block)
        .take_while(|i| *i != before)
        .filter(|i| f.dfg.insts[*i].opcode() == O::Trapnz);
    let first = traps.next().ok_or_else(invariant)?;
    let second = traps.next().ok_or_else(invariant)?;
    let pieces = args(f, f.dfg.inst_args(first)[0], O::Bor, 2)?;
    let left = args(f, pieces[0], O::Bor, 2)?;
    let right = args(f, pieces[1], O::Bor, 2)?;
    limit(f, left[0], attempts, A)?;
    let coupling = compare(f, left[1], IntCC::UnsignedGreaterThan)?;
    if *coupling != [live, attempts] {
        return Err(invariant());
    }
    limit(f, right[0], spent, S)?;
    limit(f, right[1], held, S)?;
    let sum = compare(f, f.dfg.inst_args(second)[0], IntCC::UnsignedGreaterThan)?;
    if args(f, sum[0], O::Iadd, 2)? != [spent, held]
        || !constant(f, sum[1], S.cast_signed())
        || !load(f, attempts, context, 0)
        || !load(f, live, context, 8)
        || !load(f, spent, context, 16)
        || !load(f, held, context, 24)
    {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn zero(f: &Function, block: Block, output: Value) -> Result<(), Diagnostic> {
    let mut lanes = 0_u8;
    for inst in f.layout.block_insts(block) {
        if let D::Store { args, offset, .. } = f.dfg.insts[inst] {
            let bit = match i32::from(offset) {
                0 => 1,
                8 => 2,
                16 => 4,
                _ => return Err(invariant()),
            };
            if args[1] != output
                || !constant(f, args[0], 0)
                || lanes & bit != 0
                || f.dfg.value_type(args[0]) != super::types::I64
            {
                return Err(invariant());
            }
            lanes |= bit;
        }
    }
    if lanes != 7 {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn find_load(
    f: &Function,
    block: Block,
    context: Value,
    offset: i32,
) -> Result<Value, Diagnostic> {
    f.layout
        .block_insts(block)
        .flat_map(|i| f.dfg.inst_results(i))
        .copied()
        .find(|v| load(f, *v, context, offset))
        .ok_or_else(invariant)
}
