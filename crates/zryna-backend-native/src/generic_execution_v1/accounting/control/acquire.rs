//! Admit an outer control birth/count only after forward and exact future debt fit.
use super::{
    A, ControlEffect, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, L, O, S, State,
    TrapCode, Value, arguments, canonical_status, checked, classify, constant, invariant, store,
    types,
};
struct Pending {
    old: State,
    next: State,
    possible: Value,
    merge: cranelift_codegen::ir::Block,
}
pub(in crate::generic_execution_v1) fn call(
    b: &mut FunctionBuilder<'_>,
    context: Value,
    function: FuncRef,
    effect: ControlEffect<'_, '_>,
    args: &[Value],
) -> Result<Value, Diagnostic> {
    if !matches!(
        effect.operation(),
        O::Allocate | O::StrongClone | O::WeakDowngrade | O::WeakClone | O::WeakUpgrade
    ) {
        return Err(invariant());
    }
    arguments(b, effect, args)?;
    let entry = b.create_block();
    b.ins().jump(entry, &[]);
    b.switch_to_block(entry);
    if effect.operation() == O::Allocate {
        let zero = constant(b, 0);
        store(b, args[2], 0, zero);
    }
    let pending = prepare(b, context, effect.operation(), args);
    let call = b.ins().call(function, args);
    let result = b.inst_results(call)[0];
    canonical_status(b, result, effect.operation());
    Ok(commit(b, context, &pending, result))
}
fn prepare(b: &mut FunctionBuilder<'_>, context: Value, operation: O, args: &[Value]) -> Pending {
    let old = checked(b, context);
    if operation != O::Allocate {
        classify::funded(b, old);
    }
    let c = classify::classify(b, operation, args);
    let next = State {
        a: b.ins().iadd(old.a, c.attempt),
        l: b.ins().iadd(old.l, c.attempt),
        s: b.ins().iadd_imm_u(old.s, 1),
        h: b.ins().iadd(old.h, c.debt),
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
    let cap = b.ins().iconst(types::I32, 2);
    b.ins().jump(merge, &[cap.into()]);
    b.switch_to_block(admitted);
    store(b, context, 0, next.a);
    store(b, context, 16, next.s);
    store(b, context, 24, next.h);
    Pending { old, next, possible: c.possible, merge }
}
fn commit(b: &mut FunctionBuilder<'_>, context: Value, p: &Pending, result: Value) -> Value {
    let ok = b.ins().icmp_imm_u(IntCC::Equal, result, 0);
    let impossible = b.ins().icmp_imm_u(IntCC::Equal, p.possible, 0);
    let bad = b.ins().band(impossible, ok);
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    let live = b.ins().select(ok, p.next.l, p.old.l);
    let held = b.ins().select(ok, p.next.h, p.old.h);
    store(b, context, 8, live);
    store(b, context, 24, held);
    b.ins().jump(p.merge, &[result.into()]);
    b.switch_to_block(p.merge);
    b.block_params(p.merge)[0]
}
