//! Constant-size actual SSA primitives.
use super::{D, Diagnostic, Function, Inst, IntCC, O, Value, invariant, types};
pub(super) fn origin(f: &Function, context: Value) -> Result<(), Diagnostic> {
    let entry = f.layout.entry_block().ok_or_else(invariant)?;
    if f.dfg.block_params(entry).last() != Some(&context) {
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
    matches!(data(f,value),Ok(D::UnaryImm{opcode:O::Iconst,imm}) if i64::from(*imm)==expected)
}
pub(super) fn compare(
    f: &Function,
    value: Value,
    expected: IntCC,
) -> Result<&[Value; 2], Diagnostic> {
    let D::IntCompare { opcode: O::Icmp, cond, args } = data(f, value)? else {
        return Err(invariant());
    };
    if *cond != expected {
        return Err(invariant());
    }
    Ok(args)
}
pub(super) fn load(f: &Function, value: Value, context: Value, offset: i32) -> bool {
    matches!(data(f,value),Ok(D::Load{opcode:O::Load,arg,offset:actual,..}) if *arg==context && i32::from(*actual)==offset && f.dfg.value_type(value)==types::I64)
}
pub(super) fn stored(
    f: &Function,
    inst: Inst,
    context: Value,
    offset: i32,
) -> Result<Value, Diagnostic> {
    let D::Store { opcode: O::Store, args, offset: actual, .. } = f.dfg.insts[inst] else {
        return Err(invariant());
    };
    if args[1] != context || i32::from(actual) != offset {
        return Err(invariant());
    }
    Ok(args[0])
}
pub(super) fn limit(
    f: &Function,
    value: Value,
    expected: Value,
    max: u64,
) -> Result<(), Diagnostic> {
    let cond = compare(f, value, IntCC::UnsignedGreaterThan)?;
    if cond[0] != expected || !constant(f, cond[1], max.cast_signed()) {
        return Err(invariant());
    }
    Ok(())
}
