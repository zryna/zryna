//! Provisional String acquisition, actual canonical entry and nonpublication on denial.
use super::{
    A, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, L, MAX_STRING_BYTES, MemFlagsData,
    O, S, State, TrapCode, Value, audit, checked, constant, invariant, status, store, types,
};
struct Pending {
    old: State,
    next: State,
    possible: Value,
    rejected: cranelift_codegen::ir::Block,
    admitted: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
}
pub(in crate::generic_execution_v1) fn string(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    effect: zryna_native_mir::generic_execution_v1::StringEffect<'_, '_>,
    args: &[Value],
) -> Result<Value, Diagnostic> {
    let operation = effect.operation();
    if !matches!(
        (operation, args.len()),
        (O::StringFromUtf8Copy | O::StringConcat, 3) | (O::StringClone, 2)
    ) {
        return Err(invariant());
    }
    let entry = b.create_block();
    b.ins().jump(entry, &[]);
    b.switch_to_block(entry);
    let length = length(b, operation, args)?;
    let output = *args.last().ok_or_else(invariant)?;
    let zero = constant(b, 0);
    for offset in [0, 8, 16] {
        store(b, output, offset, zero);
    }
    let pending = prepare(b, context, length);
    let call = b.ins().call(function, args);
    let result = b.inst_results(call)[0];
    status(b, result, operation);
    let merged = commit(b, context, &pending, result);
    audit::acquire(
        b.func,
        context,
        call,
        merged,
        operation,
        (pending.rejected, pending.admitted, pending.merge),
    )?;
    Ok(merged)
}
fn length(b: &mut FunctionBuilder<'_>, operation: O, args: &[Value]) -> Result<Value, Diagnostic> {
    Ok(match operation {
        O::StringFromUtf8Copy => args[1],
        O::StringClone => b.ins().load(types::I64, MemFlagsData::new(), args[0], 8),
        O::StringConcat => {
            let left = b.ins().load(types::I64, MemFlagsData::new(), args[0], 8);
            let right = b.ins().load(types::I64, MemFlagsData::new(), args[1], 8);
            // Two issued valid lengths sum to at most2*MAX_STRING_BYTES, so no u64 wrap.
            b.ins().iadd(left, right)
        }
        _ => return Err(invariant()),
    })
}
fn prepare(b: &mut FunctionBuilder<'_>, context: Value, length: Value) -> Pending {
    let old = checked(b, context);
    let possible =
        b.ins().icmp_imm_u(IntCC::UnsignedLessThanOrEqual, length, MAX_STRING_BYTES.cast_signed());
    let nonempty = b.ins().icmp_imm_u(IntCC::NotEqual, length, 0);
    let positive = b.ins().band(possible, nonempty);
    let attempt = b.ins().uextend(types::I64, positive);
    let debt = b.ins().uextend(types::I64, possible);
    let next = State {
        a: b.ins().iadd(old.a, attempt),
        l: b.ins().iadd(old.l, attempt),
        s: b.ins().iadd_imm_u(old.s, 1),
        h: b.ins().iadd(old.h, debt),
    };
    let total = b.ins().iadd(next.s, next.h);
    let statuses = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, total, S.cast_signed());
    let attempts = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, next.a, A.cast_signed());
    let lives = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, next.l, L.cast_signed());
    let allocations = b.ins().bor(attempts, lives);
    let denied = b.ins().bor(statuses, allocations);
    let rejected = b.create_block();
    let admitted = b.create_block();
    let merge = b.create_block();
    b.append_block_param(merge, types::I32);
    b.ins().brif(denied, rejected, &[], admitted, &[]);
    b.switch_to_block(rejected);
    let capacity = b.ins().iconst(types::I32, 2);
    b.ins().jump(merge, &[capacity.into()]);
    b.switch_to_block(admitted);
    store(b, context, 0, next.a);
    store(b, context, 16, next.s);
    store(b, context, 24, next.h);
    Pending { old, next, possible, rejected, admitted, merge }
}
fn commit(b: &mut FunctionBuilder<'_>, context: Value, pending: &Pending, result: Value) -> Value {
    let ok = b.ins().icmp_imm_u(IntCC::Equal, result, 0);
    let impossible = b.ins().icmp_imm_u(IntCC::Equal, pending.possible, 0);
    let impossible_ok = b.ins().band(impossible, ok);
    b.ins().trapnz(impossible_ok, TrapCode::unwrap_user(7));
    let live = b.ins().select(ok, pending.next.l, pending.old.l);
    let held = b.ins().select(ok, pending.next.h, pending.old.h);
    store(b, context, 8, live);
    store(b, context, 24, held);
    b.ins().jump(pending.merge, &[result.into()]);
    b.switch_to_block(pending.merge);
    b.block_params(pending.merge)[0]
}
