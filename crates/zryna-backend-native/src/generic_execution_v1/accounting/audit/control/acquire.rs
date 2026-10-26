//! Exact prospective debt classification from actual canonical operands.
use super::{
    Block, D, Diagnostic, Function, Inst, IntCC, LogicalOperation, O, Value, args, common, compare,
    constant, data, flow, invariant, origin, outcomes, state, types,
};
use zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES;
pub(in crate::generic_execution_v1::accounting::audit) fn check(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &Function,
    context: Value,
    call: Inst,
    merged: Value,
    operation: LogicalOperation,
    blocks: (Block, Block, Block),
) -> Result<(), Diagnostic> {
    let (rejected, admitted, merge) = blocks;
    origin(f, context)?;
    if f.layout.inst_block(call) != Some(admitted) {
        return Err(invariant());
    }
    let q = common::candidates(f, context, call)?;
    let guard = f.layout.prev_block(rejected).ok_or_else(invariant)?;
    let possible = classify(p, f, call, operation, q.attempt, q.debt, guard)?;
    let temporary = flow::admission(f, context, (q.a, q.s, q.h), q.attempt, rejected, admitted)?;
    let old_l = args(f, temporary, O::Iadd, 2)?[0];
    state::validation(f, context, guard, call, (q.old_a, old_l, q.old_s, q.old_h))?;
    let operands = f.dfg.inst_args(call);
    common::guard(f, guard, (operation == LogicalOperation::Allocate).then(|| operands[2]))?;
    if operation != LogicalOperation::Allocate {
        common::funded(f, guard, context, old_l, q.old_h)?;
    }
    outcomes::check(f, call, operation, possible)?;
    flow::results(
        f,
        context,
        call,
        merged,
        (rejected, admitted, merge),
        (temporary, q.h, q.old_a, q.old_s, q.old_h),
    )
}
fn classify(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &Function,
    call: Inst,
    operation: LogicalOperation,
    attempt: Value,
    debt: Value,
    guard: Block,
) -> Result<Value, Diagnostic> {
    let operands = f.dfg.inst_args(call);
    if operation == LogicalOperation::Allocate {
        if operands.len() != 3 {
            return Err(invariant());
        }
        geometry(p, f, operands)?;
        let both = args(f, debt, O::Iadd, 2)?;
        if *both != [attempt, attempt] {
            return Err(invariant());
        }
        let possible = args(f, attempt, O::Uextend, 1)?[0];
        let bounds = compare(f, possible, IntCC::UnsignedLessThanOrEqual)?;
        if bounds[0] != operands[0]
            || !constant(f, bounds[1], MAX_DYNAMIC_ALLOCATION_BYTES.cast_signed())
        {
            return Err(invariant());
        }
        return Ok(possible);
    }
    if operands.len() != 1 || !constant(f, attempt, 0) {
        return Err(invariant());
    }
    let possible = args(f, debt, O::Uextend, 1)?[0];
    let room = if operation == LogicalOperation::WeakUpgrade {
        let conditions = args(f, possible, O::Band, 2)?;
        let alive = compare(f, conditions[1], IntCC::NotEqual)?;
        let nonmax = compare(f, conditions[0], IntCC::NotEqual)?;
        if alive[0] != nonmax[0] || !constant(f, alive[1], 0) {
            return Err(invariant());
        }
        conditions[0]
    } else {
        possible
    };
    // Cranelift stores the canonical I32 maximum as raw4294967295, displayed as -1.
    let count = compare(f, room, IntCC::NotEqual)?;
    let offset =
        if matches!(operation, LogicalOperation::WeakDowngrade | LogicalOperation::WeakClone) {
            4
        } else {
            0
        };
    if !matches!(
        operation,
        LogicalOperation::StrongClone
            | LogicalOperation::WeakDowngrade
            | LogicalOperation::WeakClone
            | LogicalOperation::WeakUpgrade
    ) || !load32(f, count[0], operands[0], offset)
        || f.dfg.value_type(count[1]) != types::I32
        || !constant(f, count[1], i64::from(u32::MAX))
    {
        return Err(invariant());
    }
    let inst = f.dfg.value_def(count[0]).inst().ok_or_else(invariant)?;
    if f.layout.inst_block(inst) != Some(guard) {
        return Err(invariant());
    }
    Ok(possible)
}
fn geometry(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &Function,
    operands: &[Value],
) -> Result<(), Diagnostic> {
    let number = |v| -> Result<u64, Diagnostic> {
        let D::UnaryImm { opcode: O::Iconst, imm } = data(f, v)? else { return Err(invariant()) };
        u64::try_from(i64::from(*imm)).map_err(|_| invariant())
    };
    let size = number(operands[0])?;
    let alignment = number(operands[1])?;
    if size < 8
        || alignment < 4
        || !p.containers().is_some_and(|p| p.matches_control_geometry(size, alignment))
        || f.dfg.value_type(operands[0]) != types::I64
        || f.dfg.value_type(operands[1]) != types::I32
    {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn load32(f: &Function, value: Value, address: Value, offset: i32) -> bool {
    matches!(data(f,value),Ok(D::Load{opcode:O::Load,arg,offset:actual,..}) if *arg==address
        && i32::from(*actual)==offset && f.dfg.value_type(value)==types::I32)
}
