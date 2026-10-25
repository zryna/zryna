//! Exact actual predicate/value identities and integer widths; no generated-body authority.
use super::{D, Diagnostic, Function, IntCC, O, Type, Value, invariant, types};
fn definition(
    function: &Function,
    value: Value,
    ty: Type,
) -> Result<cranelift_codegen::ir::Inst, Diagnostic> {
    let inst = function.dfg.value_def(value).inst().ok_or_else(invariant)?;
    if function.dfg.inst_results(inst) != [value] || function.dfg.value_type(value) != ty {
        return Err(invariant());
    }
    Ok(inst)
}
pub(super) fn binary(
    function: &Function,
    value: Value,
    opcode: O,
    ty: Type,
) -> Result<[Value; 2], Diagnostic> {
    let inst = definition(function, value, ty)?;
    let args = function.dfg.inst_args(inst);
    if function.dfg.insts[inst].opcode() != opcode || args.len() != 2 {
        return Err(invariant());
    }
    Ok([args[0], args[1]])
}
pub(super) fn unary(
    function: &Function,
    value: Value,
    opcode: O,
    ty: Type,
) -> Result<Value, Diagnostic> {
    let inst = definition(function, value, ty)?;
    let args = function.dfg.inst_args(inst);
    if function.dfg.insts[inst].opcode() != opcode || args.len() != 1 {
        return Err(invariant());
    }
    Ok(args[0])
}
pub(super) fn code(
    function: &Function,
    predicate: Value,
    actual: Value,
    expected: i64,
) -> Result<(), Diagnostic> {
    let inst = definition(function, predicate, types::I8)?;
    let D::IntCompare { opcode: O::Icmp, cond: IntCC::Equal, args } = function.dfg.insts[inst]
    else {
        return Err(invariant());
    };
    if args[0] != actual {
        return Err(invariant());
    }
    let number = definition(function, args[1], function.dfg.value_type(actual))?;
    if !matches!(function.dfg.insts[number],D::UnaryImm{opcode:O::Iconst,imm}if i64::from(imm)==expected)
    {
        return Err(invariant());
    }
    Ok(())
}
