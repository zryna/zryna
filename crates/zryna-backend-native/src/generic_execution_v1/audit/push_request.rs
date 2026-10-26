//! Independent constant-size SSA role proof before trusted machine compilation.
use super::super::{body::Frame, invariant, runtime};
use cranelift_codegen::ir::{
    Function, Inst, InstructionData as D, Opcode as O, Value, condcodes::IntCC, types,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedVecPush;
use zryna_ownership_runtime_abi::LogicalOperation;

pub(in crate::generic_execution_v1) fn check(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedVecPush<'_, '_>,
    target: Value,
    status: Value,
    published: Inst,
) -> Result<(), Diagnostic> {
    let plan = c.p.containers().ok_or_else(invariant)?;
    let element = plan.element(view.ty()).ok_or_else(invariant)?;
    if element.element().index() != view.element_type()
        || element
            .stride()
            .checked_mul(4)
            .is_none_or(|n| n > zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES)
    {
        return Err(invariant());
    }
    let source = if let Some(place) = view.target_place() {
        zryna_ir::generic_v1::execution_v1::raw::PlaceOrLoan::Place(place.id())
    } else {
        zryna_ir::generic_v1::execution_v1::raw::PlaceOrLoan::Loan(
            view.target_loan().ok_or_else(invariant)?.id(),
        )
    };
    super::address::source(c, f, target, source)?;
    let call = f.dfg.value_def(status).inst().ok_or_else(invariant)?;
    let D::Call { func_ref, .. } = f.dfg.insts[call] else {
        return Err(invariant());
    };
    let index = runtime::NAMES.len()
        + plan
            .operations()
            .iter()
            .position(|op| *op == LogicalOperation::VecReserve)
            .ok_or_else(invariant)?;
    if c.runtime.get(index) != Some(&func_ref) {
        return Err(invariant());
    }
    let operands = f.dfg.inst_args(call);
    if operands.len() != 4
        || operands[1] != target
        || !constant(f, operands[0], i64::from(element.element().index()))
    {
        return Err(invariant());
    }
    shape(f, target, operands[2], operands[3], published)
}
fn shape(
    f: &Function,
    target: Value,
    request: Value,
    out: Value,
    published: Inst,
) -> Result<(), Diagnostic> {
    let choose = args(f, request, O::Select, 3)?;
    let empty = compare(f, choose[0], IntCC::Equal)?;
    if !load(f, empty[0], target, 16) || !constant(f, empty[1], 0) {
        return Err(invariant());
    }
    let actual = choose[2];
    let add = args(f, actual, O::Iadd, 2)?;
    if !load(f, add[0], target, 8) || !constant(f, add[1], 1) {
        return Err(invariant());
    }
    let minimum = args(f, choose[1], O::Select, 3)?;
    let smaller = compare(f, minimum[0], IntCC::UnsignedLessThan)?;
    if smaller[0] != actual
        || !constant(f, smaller[1], 4)
        || !constant(f, minimum[1], 4)
        || minimum[2] != actual
    {
        return Err(invariant());
    }
    let D::Store { opcode: O::Store, args, offset, .. } = f.dfg.insts[published] else {
        return Err(invariant());
    };
    if args != [actual, out] || i32::from(offset) != 8 {
        return Err(invariant());
    }
    Ok(())
}
fn data(f: &Function, value: Value) -> Result<&D, Diagnostic> {
    Ok(&f.dfg.insts[f.dfg.value_def(value).inst().ok_or_else(invariant)?])
}
fn args(f: &Function, value: Value, opcode: O, count: usize) -> Result<&[Value], Diagnostic> {
    let inst = f.dfg.value_def(value).inst().ok_or_else(invariant)?;
    let args = f.dfg.inst_args(inst);
    if f.dfg.insts[inst].opcode() != opcode || args.len() != count {
        return Err(invariant());
    }
    Ok(args)
}
fn constant(f: &Function, value: Value, expected: i64) -> bool {
    matches!(data(f,value),Ok(D::UnaryImm{opcode:O::Iconst,imm}) if i64::from(*imm)==expected)
}
fn compare(f: &Function, value: Value, expected: IntCC) -> Result<&[Value; 2], Diagnostic> {
    let D::IntCompare { opcode: O::Icmp, cond, args } = data(f, value)? else {
        return Err(invariant());
    };
    if *cond != expected {
        return Err(invariant());
    }
    Ok(args)
}
fn load(f: &Function, value: Value, target: Value, expected: i32) -> bool {
    matches!(data(f,value),Ok(D::Load{opcode:O::Load,arg,offset,..}) if *arg==target && i32::from(*offset)==expected && f.dfg.value_type(value)==types::I64)
}
#[cfg(test)]
mod tests;
