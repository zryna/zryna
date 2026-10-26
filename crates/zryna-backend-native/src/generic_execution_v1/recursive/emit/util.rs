//! Primitive fixed-width operations; no source/runtime permission is created here.
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{
    Block, FuncRef, InstBuilder, MemFlagsData, TrapCode, Type, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
pub(in crate::generic_execution_v1::recursive) fn c32(
    b: &mut FunctionBuilder<'_>,
    n: u32,
) -> Value {
    b.ins().iconst(types::I32, i64::from(n))
}
pub(in crate::generic_execution_v1::recursive) fn c64(
    b: &mut FunctionBuilder<'_>,
    n: u64,
) -> Value {
    b.ins().iconst(types::I64, n.cast_signed())
}
pub(in crate::generic_execution_v1::recursive) fn load(
    b: &mut FunctionBuilder<'_>,
    ty: Type,
    p: Value,
    offset: i32,
) -> Value {
    b.ins().load(ty, MemFlagsData::new(), p, offset)
}
pub(in crate::generic_execution_v1::recursive) fn store(
    b: &mut FunctionBuilder<'_>,
    p: Value,
    offset: i32,
    v: Value,
) {
    b.ins().store(MemFlagsData::new(), v, p, offset);
}
pub(in crate::generic_execution_v1::recursive) fn cmp(
    b: &mut FunctionBuilder<'_>,
    cc: IntCC,
    a: Value,
    v: Value,
) -> Value {
    b.ins().icmp(cc, a, v)
}
pub(in crate::generic_execution_v1::recursive) fn trap(b: &mut FunctionBuilder<'_>, bad: Value) {
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
}
pub(in crate::generic_execution_v1::recursive) fn call(
    b: &mut FunctionBuilder<'_>,
    function: FuncRef,
    args: &[Value],
) -> Value {
    let i = b.ins().call(function, args);
    b.inst_results(i)[0]
}
pub(in crate::generic_execution_v1::recursive) fn offset(
    b: &mut FunctionBuilder<'_>,
    p: Value,
    n: Value,
) -> Value {
    let n = b.ins().uextend(types::I64, n);
    b.ins().iadd(p, n)
}
pub(in crate::generic_execution_v1::recursive) fn add(
    b: &mut FunctionBuilder<'_>,
    p: Value,
    n: u64,
) -> Value {
    let n = c64(b, n);
    b.ins().iadd(p, n)
}
pub(in crate::generic_execution_v1::recursive) fn packed(
    b: &mut FunctionBuilder<'_>,
    child: Value,
    off: Value,
) -> Value {
    let high = c64(b, 0xffff_ffff_0000_0000);
    let high = b.ins().band(child, high);
    let low = b.ins().uextend(types::I64, off);
    b.ins().bor(high, low)
}
pub(in crate::generic_execution_v1::recursive) fn none(b: &mut FunctionBuilder<'_>) {
    let n = c64(b, u64::MAX);
    b.ins().return_(&[n]);
}
pub(in crate::generic_execution_v1::recursive) fn done(b: &mut FunctionBuilder<'_>) {
    let n = c32(b, 0);
    b.ins().return_(&[n]);
}
pub(in crate::generic_execution_v1::recursive) fn size(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    id: u32,
) -> Result<u32, Diagnostic> {
    u32::try_from(p.stored_type(id).ok_or_else(invariant)?.size()).map_err(|_| invariant())
}
pub(in crate::generic_execution_v1::recursive) fn block(
    b: &mut FunctionBuilder<'_>,
    types: &[Type],
) -> Block {
    let v = b.create_block();
    for ty in types {
        b.append_block_param(v, *ty);
    }
    v
}
