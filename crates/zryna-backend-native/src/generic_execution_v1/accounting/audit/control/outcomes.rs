//! Actual canonical status sets stay distinct from the source CAPACITY denial edge.
use super::{
    Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, args, compare, constant,
    invariant,
};
pub(super) fn check(
    f: &Function,
    call: Inst,
    operation: LogicalOperation,
    possible: Value,
) -> Result<(), Diagnostic> {
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let result = f.dfg.inst_results(call)[0];
    let mut seen = false;
    let mut valid = false;
    let mut impossible = false;
    for inst in f.layout.block_insts(block) {
        if inst == call {
            seen = true;
            continue;
        }
        if !seen {
            continue;
        }
        if f.dfg.insts[inst].opcode() == O::Trapz {
            let pieces = args(f, f.dfg.inst_args(inst)[0], O::Bor, 2)?;
            valid = if operation == LogicalOperation::Allocate
                || operation == LogicalOperation::WeakUpgrade
            {
                let first = args(f, pieces[0], O::Bor, 2)?;
                let (a, b) = if operation == LogicalOperation::Allocate { (1, 2) } else { (3, 5) };
                code(f, first[0], result, 0)
                    && code(f, first[1], result, a)
                    && code(f, pieces[1], result, b)
            } else {
                code(f, pieces[0], result, 0) && code(f, pieces[1], result, 3)
            };
        }
        if f.dfg.insts[inst].opcode() == O::Trapnz {
            let both = args(f, f.dfg.inst_args(inst)[0], O::Band, 2)?;
            impossible = code(f, both[0], possible, 0) && code(f, both[1], result, 0);
        }
    }
    if !valid || !impossible {
        return Err(invariant());
    }
    Ok(())
}
fn code(f: &Function, value: Value, actual: Value, expected: i64) -> bool {
    compare(f, value, IntCC::Equal).is_ok_and(|a| a[0] == actual && constant(f, a[1], expected))
}
