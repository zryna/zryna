//! Mandatory count cleanup spends held credit and cannot source-quota deny.
use super::{
    ControlEffect, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, MemFlagsData, O,
    TrapCode, Value, arguments, checked, classify, constant, invariant, store, types,
};
mod facts;
pub(in crate::generic_execution_v1) fn release(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    effect: ControlEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    arguments(b, effect, args)?;
    let entry = b.create_block();
    b.ins().jump(entry, &[]);
    b.switch_to_block(entry);
    let facts = facts::capture(b, effect.operation(), args)?;
    let old = checked(b, context);
    classify::funded(b, old);
    let spent = b.ins().iadd_imm_u(old.s, 1);
    let held = b.ins().iadd_imm_s(old.h, -1);
    store(b, context, 16, spent);
    store(b, context, 24, held);
    let call = b.ins().call(function, args);
    let result = b.inst_results(call)[0];
    b.ins().trapnz(result, TrapCode::unwrap_user(7));
    facts::validate(b, &facts, args);
    let live = b.ins().isub(old.l, facts.amount);
    store(b, context, 8, live);
    Ok(())
}
