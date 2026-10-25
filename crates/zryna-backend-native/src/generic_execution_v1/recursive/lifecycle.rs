//! Root invocation lease; no arena reset, counter refresh, allocation or release effect.
use super::emit::util::{c32, c64, call, cmp, load, store, trap};
use cranelift_codegen::ir::{FuncRef, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
pub(in crate::generic_execution_v1) fn begin(
    b: &mut FunctionBuilder<'_>,
    getter: FuncRef,
    ctx: Value,
) -> Value {
    let base = call(b, getter, &[ctx]);
    let used = load(b, types::I32, base, 0);
    let busy = load(b, types::I32, base, 4);
    let owner = load(b, types::I64, base, 8);
    let zero = c32(b, 0);
    let used = cmp(b, IntCC::NotEqual, used, zero);
    let busy = cmp(b, IntCC::NotEqual, busy, zero);
    let null = c64(b, 0);
    let owner = cmp(b, IntCC::NotEqual, owner, null);
    let bad = b.ins().bor(used, busy);
    let bad = b.ins().bor(bad, owner);
    trap(b, bad);
    let one = c32(b, 1);
    store(b, base, 4, one);
    store(b, base, 8, ctx);
    base
}
pub(in crate::generic_execution_v1) fn completed(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    ctx: Value,
) {
    let used = load(b, types::I32, base, 0);
    let busy = load(b, types::I32, base, 4);
    let owner = load(b, types::I64, base, 8);
    let zero = c32(b, 0);
    let used = cmp(b, IntCC::NotEqual, used, zero);
    let one = c32(b, 1);
    let busy = cmp(b, IntCC::NotEqual, busy, one);
    let owner = cmp(b, IntCC::NotEqual, owner, ctx);
    let bad = b.ins().bor(used, busy);
    let bad = b.ins().bor(bad, owner);
    trap(b, bad);
}
pub(in crate::generic_execution_v1) fn end(b: &mut FunctionBuilder<'_>, base: Value) {
    let null = c64(b, 0);
    store(b, base, 8, null);
    let zero = c32(b, 0);
    store(b, base, 4, zero);
}
