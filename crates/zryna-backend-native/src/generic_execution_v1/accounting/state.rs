//! Fixed scalar counter state, validity checks and infallible held-credit release helpers.
use super::{
    A, FunctionBuilder, InstBuilder, IntCC, MemFlagsData, O, S, State, TrapCode, Value, types,
};
pub(super) fn constant(b: &mut FunctionBuilder<'_>, n: u64) -> Value {
    b.ins().iconst(types::I64, n.cast_signed())
}
pub(super) fn store(b: &mut FunctionBuilder<'_>, context: Value, offset: i32, value: Value) {
    b.ins().store(MemFlagsData::new(), value, context, offset);
}
pub(super) fn load(b: &mut FunctionBuilder<'_>, context: Value) -> State {
    let attempts = b.ins().load(types::I64, MemFlagsData::new(), context, 0);
    let live = b.ins().load(types::I64, MemFlagsData::new(), context, 8);
    let spent = b.ins().load(types::I64, MemFlagsData::new(), context, 16);
    let held = b.ins().load(types::I64, MemFlagsData::new(), context, 24);
    State { a: attempts, l: live, s: spent, h: held }
}
pub(super) fn checked(b: &mut FunctionBuilder<'_>, context: Value) -> State {
    let snapshot = load(b, context);
    let attempts = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, snapshot.a, A.cast_signed());
    let live = b.ins().icmp(IntCC::UnsignedGreaterThan, snapshot.l, snapshot.a);
    let spent = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, snapshot.s, S.cast_signed());
    let held = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, snapshot.h, S.cast_signed());
    let left = b.ins().bor(attempts, live);
    let right = b.ins().bor(spent, held);
    let bad = b.ins().bor(left, right);
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    // Each summand is now bounded by S, so this sum cannot wrap.
    let total = b.ins().iadd(snapshot.s, snapshot.h);
    let bad = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, total, S.cast_signed());
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    snapshot
}
pub(super) fn status(b: &mut FunctionBuilder<'_>, value: Value, operation: O) {
    let mut valid = b.ins().icmp_imm_u(IntCC::Equal, value, 0);
    for code in [1, 2] {
        let bit = b.ins().icmp_imm_u(IntCC::Equal, value, code);
        valid = b.ins().bor(valid, bit);
    }
    if operation == O::StringFromUtf8Copy {
        let bit = b.ins().icmp_imm_u(IntCC::Equal, value, 4);
        valid = b.ins().bor(valid, bit);
    }
    b.ins().trapz(valid, TrapCode::unwrap_user(7));
}
