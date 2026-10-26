//! Independently bind actual source addresses to checked frame/formal/prepared geometry.
use super::super::{body::Frame, invariant};
use cranelift_codegen::ir::{Function, InstructionData as D, Opcode as O, Value, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::raw;
use zryna_native_mir::generic_execution_v1::LoanAddress;
pub(super) fn source(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    address: Value,
    source: raw::PlaceOrLoan,
) -> Result<(), Diagnostic> {
    let loan = match source {
        raw::PlaceOrLoan::Place(id) => LoanAddress::Frame(c.f.place(id).ok_or_else(invariant)?),
        raw::PlaceOrLoan::Loan(id) => c.f.loan(id).ok_or_else(invariant)?,
    };
    let parts = args(f, address, O::Iadd, 2)?;
    match loan {
        LoanAddress::Frame(record) => sum(f, parts, c.base, record.offset()),
        LoanAddress::Formal { parameter, offset } => {
            sum(f, parts, *c.parameters.get(parameter as usize + 1).ok_or_else(invariant)?, offset)
        }
        LoanAddress::Prepared { slot, offset } => {
            if !constant(f, parts[1], i64::from(offset)) {
                return Err(invariant());
            }
            let D::Load { opcode: O::Load, arg, offset: at, .. } = data(f, parts[0])? else {
                return Err(invariant());
            };
            if i32::from(*at) != 0 || f.dfg.value_type(parts[0]) != types::I64 {
                return Err(invariant());
            }
            sum(f, args(f, *arg, O::Iadd, 2)?, c.base, slot)
        }
    }
}
pub(super) fn cell(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    address: Value,
    slot: u32,
) -> Result<(), Diagnostic> {
    if slot.checked_add(8).is_none_or(|end| end > c.f.frame_bytes()) || !slot.is_multiple_of(8) {
        return Err(invariant());
    }
    sum(f, args(f, address, O::Iadd, 2)?, c.base, slot)
}
fn sum(f: &Function, parts: &[Value], base: Value, offset: u32) -> Result<(), Diagnostic> {
    if parts[0] != base || !constant(f, parts[1], i64::from(offset)) {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn data(f: &Function, value: Value) -> Result<&D, Diagnostic> {
    Ok(&f.dfg.insts[f.dfg.value_def(value).inst().ok_or_else(invariant)?])
}
pub(super) fn args(
    f: &Function,
    value: Value,
    opcode: O,
    count: usize,
) -> Result<&[Value], Diagnostic> {
    let inst = f.dfg.value_def(value).inst().ok_or_else(invariant)?;
    let values = f.dfg.inst_args(inst);
    if f.dfg.insts[inst].opcode() != opcode || values.len() != count {
        return Err(invariant());
    }
    Ok(values)
}
pub(super) fn constant(f: &Function, value: Value, expected: i64) -> bool {
    matches!(data(f,value), Ok(D::UnaryImm { opcode: O::Iconst, imm }) if i64::from(*imm) == expected)
}
