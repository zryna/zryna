//! Preflight before canonical Vec side effects; status2 denial does not enter C.
use super::{
    A, Call, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, L, MemFlagsData, O, S,
    State, TrapCode, Value, VectorEffect, checked, classify, constant, element, invariant, status,
    store, types,
};
struct Pending {
    old: State,
    next: State,
    classified: classify::Classification,
    merge: cranelift_codegen::ir::Block,
}
pub(in crate::generic_execution_v1) fn call(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    effect: VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<Call, Diagnostic> {
    element(b, effect, args)?;
    let entry = b.create_block();
    b.ins().jump(entry, &[]);
    b.switch_to_block(entry);
    let output = *args.last().ok_or_else(invariant)?;
    let zero = constant(b, 0);
    for offset in [0, 8, 16] {
        store(b, output, offset, zero);
    }
    let pending = prepare(b, context, effect, args)?;
    let call = b.ins().call(function, args);
    let canonical = b.inst_results(call)[0];
    status(b, canonical, effect.operation());
    Ok(commit(b, context, &pending, canonical))
}
fn prepare(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    effect: VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<Pending, Diagnostic> {
    let old = checked(b, context);
    funded(b, old, effect, args);
    let classified = classify::classify(b, effect, args)?;
    let next = State {
        a: b.ins().iadd(old.a, classified.attempt),
        l: b.ins().iadd(old.l, classified.attempt),
        s: b.ins().iadd_imm_u(old.s, 1),
        h: b.ins().iadd(old.h, classified.debt),
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
    Ok(Pending { old, next, classified, merge })
}
fn commit(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    pending: &Pending,
    canonical: Value,
) -> Call {
    let ok = b.ins().icmp_imm_u(IntCC::Equal, canonical, 0);
    let impossible = b.ins().icmp_imm_u(IntCC::Equal, pending.classified.possible, 0);
    let impossible_ok = b.ins().band(impossible, ok);
    b.ins().trapnz(impossible_ok, TrapCode::unwrap_user(7));
    let after = b.ins().isub(pending.next.l, pending.classified.retired);
    let live = b.ins().select(ok, after, pending.old.l);
    let held = b.ins().select(ok, pending.next.h, pending.old.h);
    store(b, context, 8, live);
    store(b, context, 24, held);
    b.ins().jump(pending.merge, &[canonical.into()]);
    b.switch_to_block(pending.merge);
    Call { status: b.block_params(pending.merge)[0], canonical }
}

fn funded(b: &mut FunctionBuilder<'_>, old: State, effect: VectorEffect<'_, '_>, args: &[Value]) {
    if effect.operation() == O::VecReserve {
        let capacity = b.ins().load(types::I64, MemFlagsData::new(), args[1], 16);
        let positive = b.ins().icmp_imm_u(IntCC::NotEqual, capacity, 0);
        let amount = b.ins().uextend(types::I64, positive);
        let missing = b.ins().icmp_imm_u(IntCC::Equal, old.h, 0);
        let absent = b.ins().icmp(IntCC::UnsignedLessThan, old.l, amount);
        let bad = b.ins().bor(missing, absent);
        b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    }
}
