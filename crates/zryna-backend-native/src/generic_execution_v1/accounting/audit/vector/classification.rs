//! Canonical element/byte ceilings and exact bounded doubling are independently decoded.
use super::{
    Block, D, Diagnostic, Function, IntCC, LogicalOperation, O, Value, args, compare, constant,
    data, invariant, load, types,
};
use zryna_ownership_runtime_abi::{MAX_DYNAMIC_ALLOCATION_BYTES, MAX_VEC_ELEMENTS};
pub(super) fn metadata(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &Function,
    operands: &[Value],
) -> Result<zryna_ownership_runtime_abi::generic_v1::ElementLayout, Diagnostic> {
    let element = *operands.first().ok_or_else(invariant)?;
    let D::UnaryImm { opcode: O::Iconst, imm } = data(f, element)? else {
        return Err(invariant());
    };
    let id = u32::try_from(i64::from(*imm)).map_err(|_| invariant())?;
    if f.dfg.value_type(element) != types::I32 {
        return Err(invariant());
    }
    p.containers().and_then(|plan| plan.element_by_identity(id)).ok_or_else(invariant)
}
pub(super) fn check(
    f: &Function,
    operands: &[Value],
    operation: LogicalOperation,
    stride: u64,
    possible: Value,
    growth: Value,
    debt: Value,
) -> Result<(), Diagnostic> {
    let bounds = compare(f, possible, IntCC::UnsignedLessThanOrEqual)?;
    let max = MAX_VEC_ELEMENTS.min(MAX_DYNAMIC_ALLOCATION_BYTES / stride);
    if !constant(f, bounds[1], max.cast_signed()) {
        return Err(invariant());
    }
    match operation {
        LogicalOperation::VecAllocate if operands.len() == 3 => {
            let nonempty = compare(f, growth, IntCC::NotEqual)?;
            if nonempty[0] != operands[1]
                || !constant(f, nonempty[1], 0)
                || bounds[0] != operands[1]
                || args(f, debt, O::Uextend, 1)? != [possible]
            {
                return Err(invariant());
            }
        }
        LogicalOperation::VecReserve if operands.len() == 4 => {
            if !constant(f, debt, 0) {
                return Err(invariant());
            }
            let growing = compare(f, growth, IntCC::UnsignedGreaterThan)?;
            if growing[0] != operands[2] || !load(f, growing[1], operands[1], 16) {
                return Err(invariant());
            }
            let capacity = args(f, bounds[0], O::Select, 3)?;
            if capacity[0] != growth || capacity[2] != growing[1] {
                return Err(invariant());
            }
            doubling(f, capacity[1], growing[1], operands[2])?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn doubling(f: &Function, capacity: Value, old: Value, required: Value) -> Result<(), Diagnostic> {
    let final_choice = args(f, capacity, O::Select, 3)?;
    let short = compare(f, final_choice[0], IntCC::UnsignedLessThan)?;
    if *short != [final_choice[2], required] || final_choice[1] != required {
        return Err(invariant());
    }
    let mut current = final_choice[2];
    for _ in 0..20 {
        let choice = args(f, current, O::Select, 3)?;
        let before = choice[2];
        let conditions = args(f, choice[0], O::Band, 2)?;
        let short = compare(f, conditions[0], IntCC::UnsignedLessThan)?;
        let small = compare(f, conditions[1], IntCC::UnsignedLessThanOrEqual)?;
        if *short != [before, required]
            || small[0] != before
            || !constant(f, small[1], (MAX_VEC_ELEMENTS / 2).cast_signed())
            || args(f, choice[1], O::Iadd, 2)? != [before, before]
        {
            return Err(invariant());
        }
        current = before;
    }
    let start = args(f, current, O::Select, 3)?;
    let empty = compare(f, start[0], IntCC::Equal)?;
    if empty[0] != old || !constant(f, empty[1], 0) || !constant(f, start[1], 1) || start[2] != old
    {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn retired(
    f: &Function,
    operation: LogicalOperation,
    operands: &[Value],
    growth: Value,
    retired: Value,
) -> Result<(), Diagnostic> {
    if operation == LogicalOperation::VecAllocate {
        return if constant(f, retired, 0) { Ok(()) } else { Err(invariant()) };
    }
    let amount = args(f, retired, O::Uextend, 1)?;
    let both = args(f, amount[0], O::Band, 2)?;
    let positive = compare(f, both[1], IntCC::NotEqual)?;
    if both[0] != growth || !load(f, positive[0], operands[1], 16) || !constant(f, positive[1], 0) {
        return Err(invariant());
    }
    Ok(())
}

pub(super) fn funded(
    f: &Function,
    guard: Block,
    context: Value,
    address: Value,
    old_l: Value,
    old_h: Value,
) -> Result<(), Diagnostic> {
    let trap = f
        .layout
        .block_insts(guard)
        .filter(|i| f.dfg.insts[*i].opcode() == O::Trapnz)
        .nth(2)
        .ok_or_else(invariant)?;
    let conditions = args(f, f.dfg.inst_args(trap)[0], O::Bor, 2)?;
    let missing = compare(f, conditions[0], IntCC::Equal)?;
    let absent = compare(f, conditions[1], IntCC::UnsignedLessThan)?;
    let amount = args(f, absent[1], O::Uextend, 1)?;
    let positive = compare(f, amount[0], IntCC::NotEqual)?;
    if missing[0] != old_h
        || !constant(f, missing[1], 0)
        || absent[0] != old_l
        || !load(f, positive[0], address, 16)
        || !constant(f, positive[1], 0)
        || !load(f, old_l, context, 8)
    {
        return Err(invariant());
    }
    Ok(())
}
