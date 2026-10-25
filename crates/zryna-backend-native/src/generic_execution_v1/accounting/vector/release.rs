//! Storage release spends one previously held credit, including capacity-zero Vec.
use super::{
    Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, MemFlagsData, O, TrapCode, Value,
    VectorEffect, checked, element, invariant, store, types,
};
pub(in crate::generic_execution_v1) fn release(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    effect: VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    if effect.operation() != O::VecReleaseStorage || args.len() != 2 {
        return Err(invariant());
    }
    element(b, effect, args)?;
    let entry = b.create_block();
    b.ins().jump(entry, &[]);
    b.switch_to_block(entry);
    let capacity = b.ins().load(types::I64, MemFlagsData::new(), args[1], 16);
    let positive = b.ins().icmp_imm_u(IntCC::NotEqual, capacity, 0);
    let amount = b.ins().uextend(types::I64, positive);
    let old = checked(b, context);
    let missing = b.ins().icmp_imm_u(IntCC::Equal, old.h, 0);
    let absent = b.ins().icmp(IntCC::UnsignedLessThan, old.l, amount);
    let bad = b.ins().bor(missing, absent);
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    let spent = b.ins().iadd_imm_u(old.s, 1);
    let held = b.ins().iadd_imm_s(old.h, -1);
    store(b, context, 16, spent);
    store(b, context, 24, held);
    let call = b.ins().call(function, args);
    let result = b.inst_results(call)[0];
    b.ins().trapnz(result, TrapCode::unwrap_user(7));
    let live = b.ins().isub(old.l, amount);
    store(b, context, 8, live);
    Ok(())
}
