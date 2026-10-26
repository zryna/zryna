//! Bind captured exact counts, actual out booleans and physical live decrement.
use super::{
    Block, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, acquire, args, compare,
    constant, invariant,
};
pub(super) fn check(
    f: &Function,
    block: Block,
    call: Inst,
    operation: LogicalOperation,
    amount: Value,
    output: Option<Value>,
) -> Result<(), Diagnostic> {
    let control = f.dfg.inst_args(call)[0];
    let predicate = if operation == LogicalOperation::StrongReleaseBegin {
        if !constant(f, amount, 0) {
            return Err(invariant());
        }
        output_predicate(f, block, call, output.ok_or_else(invariant)?)?
    } else {
        let predicate = args(f, amount, O::Uextend, 1)?[0];
        if let Some(output) = output
            && output_predicate(f, block, call, output)? != predicate
        {
            return Err(invariant());
        }
        predicate
    };
    match operation {
        LogicalOperation::StrongReleaseBegin => count(f, block, call, predicate, control, 0, 1)?,
        LogicalOperation::StrongReleaseFinish => count(f, block, call, predicate, control, 4, 1)?,
        LogicalOperation::WeakRelease => {
            let both = args(f, predicate, O::Band, 2)?;
            count(f, block, call, both[0], control, 0, 0)?;
            count(f, block, call, both[1], control, 4, 1)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn count(
    f: &Function,
    block: Block,
    call: Inst,
    predicate: Value,
    control: Value,
    offset: i32,
    value: i64,
) -> Result<(), Diagnostic> {
    let parts = compare(f, predicate, IntCC::Equal)?;
    let inst = f.dfg.value_def(parts[0]).inst().ok_or_else(invariant)?;
    if !acquire::load32(f, parts[0], control, offset)
        || !constant(f, parts[1], value)
        || !f.layout.block_insts(block).take_while(|i| *i != call).any(|i| i == inst)
    {
        return Err(invariant());
    }
    Ok(())
}
fn output_predicate(
    f: &Function,
    block: Block,
    call: Inst,
    output: Value,
) -> Result<Value, Diagnostic> {
    let mut after = false;
    for inst in f.layout.block_insts(block) {
        if inst == call {
            after = true;
            continue;
        }
        if !after || f.dfg.insts[inst].opcode() != O::Trapnz {
            continue;
        }
        let condition = f.dfg.inst_args(inst)[0];
        let Ok(parts) = compare(f, condition, IntCC::NotEqual) else { continue };
        if !acquire::load32(f, parts[0], output, 0) {
            return Err(invariant());
        }
        return Ok(args(f, parts[1], O::Uextend, 1)?[0]);
    }
    Err(invariant())
}
