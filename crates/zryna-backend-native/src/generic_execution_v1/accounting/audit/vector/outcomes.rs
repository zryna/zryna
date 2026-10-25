//! Allowed canonical statuses and impossible-success trap are actual SSA obligations.
use super::{Diagnostic, Function, Inst, IntCC, O, Value, args, compare, constant, invariant};
pub(super) fn check(f: &Function, call: Inst, possible: Value) -> Result<(), Diagnostic> {
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let result = *f.dfg.inst_results(call).first().ok_or_else(invariant)?;
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
            let bits = args(f, f.dfg.inst_args(inst)[0], O::Bor, 2)?;
            let first = args(f, bits[0], O::Bor, 2)?;
            valid = code(f, first[0], result, 0)
                && code(f, first[1], result, 1)
                && code(f, bits[1], result, 2);
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
fn code(f: &Function, value: Value, actual: Value, code: i64) -> bool {
    compare(f, value, IntCC::Equal)
        .is_ok_and(|parts| parts[0] == actual && constant(f, parts[1], code))
}
