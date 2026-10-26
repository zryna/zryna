//! Independent complete function context census and exact generated-role dispatch.
use super::{
    Block, D, Diagnostic, FuncRef, Function, Inst, LogicalOperation, Value, acquire, control,
    invariant, origin, release, vector,
};
use crate::generic_execution_v1::runtime;
type Program<'p, 'a> = zryna_native_mir::generic_execution_v1::VerifiedProgram<'p, 'a>;
pub(in crate::generic_execution_v1::accounting) fn check(
    p: &Program<'_, '_>,
    f: &Function,
    context: Value,
    runtime: &[FuncRef],
) -> Result<(), Diagnostic> {
    origin(f, context)?;
    for block in f.layout.blocks() {
        let classified = f.layout.block_insts(block).any(|inst| {
            matches!(f.dfg.insts[inst],
            D::Call{func_ref,..} if role(p,runtime,func_ref).is_some())
        });
        for inst in f.layout.block_insts(block) {
            context_use(f, context, runtime, inst, classified)?;
            if let D::Call { func_ref, .. } = f.dfg.insts[inst] {
                verify(p, f, context, runtime, inst, func_ref)?;
            }
        }
    }
    Ok(())
}
fn role(p: &Program<'_, '_>, refs: &[FuncRef], function: FuncRef) -> Option<LogicalOperation> {
    runtime::string_operation(p, refs, function)
        .or_else(|| runtime::vector_operation(p, refs, function))
        .or_else(|| runtime::control_operation(p, refs, function))
}
fn context_use(
    f: &Function,
    context: Value,
    runtime: &[FuncRef],
    inst: Inst,
    classified: bool,
) -> Result<(), Diagnostic> {
    if !f.dfg.inst_args(inst).contains(&context) {
        return Ok(());
    }
    match f.dfg.insts[inst] {
        D::Load { arg, offset, .. }
            if arg == context && [0, 8, 16, 24].contains(&i32::from(offset)) => {}
        D::Store { args, .. } if classified && args[1] == context && args[0] != context => {}
        D::Call { func_ref, .. } if !runtime.contains(&func_ref) => {}
        _ => return Err(invariant()),
    }
    Ok(())
}
fn verify(
    p: &Program<'_, '_>,
    f: &Function,
    context: Value,
    refs: &[FuncRef],
    inst: Inst,
    function: FuncRef,
) -> Result<(), Diagnostic> {
    let Some(operation) = role(p, refs, function) else {
        if !refs.contains(&function) {
            let operands = f.dfg.inst_args(inst);
            if operands.last() != Some(&context)
                || operands[..operands.len() - 1].contains(&context)
            {
                return Err(invariant());
            }
        }
        return Ok(());
    };
    match operation {
        LogicalOperation::StringRelease => release(f, context, inst, f.dfg.inst_args(inst)[0]),
        LogicalOperation::VecReleaseStorage => vector::storage(p, f, context, inst),
        LogicalOperation::StrongReleaseBegin
        | LogicalOperation::StrongReleaseFinish
        | LogicalOperation::WeakRelease => control::release(f, context, inst, operation),
        _ => {
            let (value, blocks) = merged(f, inst)?;
            if matches!(
                operation,
                LogicalOperation::StringFromUtf8Copy
                    | LogicalOperation::StringClone
                    | LogicalOperation::StringConcat
            ) {
                acquire(f, context, inst, value, operation, blocks)
            } else if matches!(
                operation,
                LogicalOperation::VecAllocate | LogicalOperation::VecReserve
            ) {
                vector::check(p, f, context, inst, value, operation, blocks)
            } else {
                control::acquire(p, f, context, inst, value, operation, blocks)
            }
        }
    }
}
fn merged(f: &Function, call: Inst) -> Result<(Value, (Block, Block, Block)), Diagnostic> {
    let block = f.layout.inst_block(call).ok_or_else(invariant)?;
    let jump = f.layout.last_inst(block).ok_or_else(invariant)?;
    let targets = f.dfg.insts[jump].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
    if targets.len() != 1 {
        return Err(invariant());
    }
    let merge = targets[0].block(&f.dfg.value_lists);
    let rejected = f.layout.prev_block(block).ok_or_else(invariant)?;
    let value = *f.dfg.block_params(merge).first().ok_or_else(invariant)?;
    Ok((value, (rejected, block, merge)))
}
