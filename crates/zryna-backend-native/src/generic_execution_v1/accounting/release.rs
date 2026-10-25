//! Mandatory String cleanup spends existing held credit and never source quota-denies.
use super::{
    Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, MemFlagsData, TrapCode, Value, audit,
    checked, store, types,
};
pub(in crate::generic_execution_v1) fn release(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    address: Value,
) -> Result<(), Diagnostic> {
    let release_entry = b.create_block();
    b.ins().jump(release_entry, &[]);
    b.switch_to_block(release_entry);
    let length = b.ins().load(types::I64, MemFlagsData::new(), address, 8);
    let positive = b.ins().icmp_imm_u(IntCC::NotEqual, length, 0);
    let amount = b.ins().uextend(types::I64, positive);
    let q = checked(b, context);
    let missing = b.ins().icmp_imm_u(IntCC::Equal, q.h, 0);
    let absent = b.ins().icmp(IntCC::UnsignedLessThan, q.l, amount);
    let bad = b.ins().bor(missing, absent);
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    let spent = b.ins().iadd_imm_u(q.s, 1);
    let held = b.ins().iadd_imm_s(q.h, -1);
    store(b, context, 16, spent);
    store(b, context, 24, held);
    let call = b.ins().call(function, &[address]);
    let status = b.inst_results(call)[0];
    b.ins().trapnz(status, TrapCode::unwrap_user(7));
    let live = b.ins().isub(q.l, amount);
    store(b, context, 8, live);
    audit::release(b.func, context, call, address)?;
    Ok(())
}
