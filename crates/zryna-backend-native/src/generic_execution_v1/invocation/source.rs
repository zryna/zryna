//! Invocation-only normalization at source-owned failure edges, after their exact cleanup.
use super::{plan::Plan, status::Cases};
use crate::generic_execution_v1::{
    body::{Frame, cleanup, failure},
    invariant,
};
use cranelift_codegen::ir::{Inst, InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCleanupPlan, VerifiedOperation, VerifiedVecPush, VerifiedWeakUpgrade,
};
pub(in crate::generic_execution_v1) fn operation(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    status: Value,
) -> Result<(), Diagnostic> {
    let cases = c.transport.map(|plan| plan.operation(op)).transpose()?;
    emit(c, b, op.failure(), status, cases)
}
pub(in crate::generic_execution_v1) fn overflow(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedVecPush<'_, '_>,
    status: Value,
) -> Result<(), Diagnostic> {
    emit(c, b, Some(view.outer_failure()), status, c.transport.map(|_| Cases::capacity()))
}
pub(in crate::generic_execution_v1) fn upgrade(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedWeakUpgrade<'_, '_>,
    status: Value,
) -> Result<(), Diagnostic> {
    emit(c, b, Some(view.outer_failure()), status, c.transport.map(|_| Cases::count()))
}
fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    plan: Option<VerifiedCleanupPlan<'_, '_>>,
    status: Value,
    cases: Option<Cases>,
) -> Result<(), Diagnostic> {
    let Some(cases) = cases else {
        return failure(c, b, plan, status);
    };
    let failed = b.create_block();
    let success = b.create_block();
    b.ins().brif(status, failed, &[], success, &[]);
    b.switch_to_block(failed);
    cleanup(c, b, plan.ok_or_else(invariant)?)?;
    returned(b, status, cases)?;
    b.switch_to_block(success);
    Ok(())
}
pub(in crate::generic_execution_v1) fn clone_return(
    b: &mut FunctionBuilder<'_>,
    plan: Option<&Plan<'_>>,
    op: VerifiedOperation<'_, '_>,
    status: Value,
) -> Result<(), Diagnostic> {
    if let Some(plan) = plan {
        returned(b, status, plan.operation(op)?)
    } else {
        b.ins().return_(&[status]);
        Ok(())
    }
}
fn returned(b: &mut FunctionBuilder<'_>, status: Value, cases: Cases) -> Result<(), Diagnostic> {
    if cases.len() == 0 {
        b.ins().return_(&[status]);
        return Ok(());
    }
    let block = b.current_block().ok_or_else(invariant)?;
    let previous = b.func.layout.last_inst(block);
    let mut mapped = b.ins().iconst(types::I32, 255);
    for (canonical, source) in cases.entries() {
        let expected = b.ins().iconst(types::I32, i64::from(canonical));
        let matches =
            b.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, status, expected);
        let code = b.ins().iconst(types::I32, i64::from(source.code()));
        mapped = b.ins().select(matches, code, mapped);
    }
    let end = b.ins().return_(&[mapped]);
    check(b.func, previous, end, status, cases)?;
    #[cfg(test)]
    trace::record(previous, end, status, cases);
    Ok(())
}
fn check(
    f: &cranelift_codegen::ir::Function,
    previous: Option<Inst>,
    end: Inst,
    status: Value,
    cases: Cases,
) -> Result<(), Diagnostic> {
    use cranelift_codegen::ir::{InstructionData as D, Opcode as O, condcodes::IntCC};
    let block = f.layout.inst_block(end).ok_or_else(invariant)?;
    let mut next = previous.map_or_else(|| f.layout.first_inst(block), |i| f.layout.next_inst(i));
    let mut take = || {
        let i = next.ok_or_else(invariant)?;
        next = f.layout.next_inst(i);
        Ok::<_, Diagnostic>(i)
    };
    let constant = |i: Inst, expected: u32| -> Result<Value, Diagnostic> {
        if !matches!(f.dfg.insts[i],D::UnaryImm{opcode:O::Iconst,imm} if i64::from(imm)==i64::from(expected))
        {
            return Err(invariant());
        }
        let values = f.dfg.inst_results(i);
        if values.len() != 1 || f.dfg.value_type(values[0]) != types::I32 {
            return Err(invariant());
        }
        Ok(values[0])
    };
    let mut mapped = constant(take()?, 255)?;
    for (canonical, source) in cases.entries() {
        let expected = constant(take()?, canonical)?;
        let compare = take()?;
        if !matches!(f.dfg.insts[compare],D::IntCompare{opcode:O::Icmp,cond:IntCC::Equal,args} if args==[status,expected])
        {
            return Err(invariant());
        }
        let matches = *f.dfg.inst_results(compare).first().ok_or_else(invariant)?;
        if f.dfg.value_type(matches) != types::I8 {
            return Err(invariant());
        }
        let code = constant(take()?, source.code())?;
        let select = take()?;
        if f.dfg.insts[select].opcode() != O::Select
            || f.dfg.inst_args(select) != [matches, code, mapped]
        {
            return Err(invariant());
        }
        mapped = *f.dfg.inst_results(select).first().ok_or_else(invariant)?;
        if f.dfg.value_type(mapped) != types::I32 {
            return Err(invariant());
        }
    }
    let returned = take()?;
    if returned != end
        || f.dfg.insts[end].opcode() != O::Return
        || f.dfg.inst_args(end) != [mapped]
        || next.is_some()
    {
        return Err(invariant());
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::generic_execution_v1) mod trace;
