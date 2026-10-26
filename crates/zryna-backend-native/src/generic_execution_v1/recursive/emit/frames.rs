//! Fixed preallocated frame lanes; current owner stays live until its frame is popped.
use super::util::{self as u, c32, c64, cmp, load, store, trap};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
#[derive(Clone, Copy)]
pub(super) struct Pending {
    pub leaf: u32,
    pub root: Value,
    pub ty: Value,
    pub offset: Value,
    pub index: Value,
    pub role: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Top {
    pub previous: Value,
    pub address: Value,
    pub leaf: Value,
    pub root: Value,
    pub ty: Value,
    pub offset: Value,
    pub index: Value,
    pub role: Value,
}
pub(super) fn address(b: &mut FunctionBuilder<'_>, arena: Value, index: Value) -> Value {
    let wide = b.ins().uextend(types::I64, index);
    let width = c64(b, 32);
    let bytes = b.ins().imul(wide, width);
    let header = u::add(b, arena, 16);
    b.ins().iadd(header, bytes)
}
pub(super) fn push(
    b: &mut FunctionBuilder<'_>,
    arena: Value,
    frame: Pending,
) -> Result<(), Diagnostic> {
    let used = load(b, types::I32, arena, 0);
    let cap = c32(
        b,
        u32::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| invariant())?,
    );
    let full = cmp(b, IntCC::UnsignedGreaterThanOrEqual, used, cap);
    trap(b, full);
    let destination = address(b, arena, used);
    let leaf = c64(b, u64::from(frame.leaf));
    store(b, destination, 0, leaf);
    store(b, destination, 8, frame.root);
    store(b, destination, 16, frame.ty);
    store(b, destination, 20, frame.offset);
    store(b, destination, 24, frame.index);
    let role = c32(b, frame.role);
    store(b, destination, 28, role);
    let one = c32(b, 1);
    let next = b.ins().iadd(used, one);
    store(b, arena, 0, next);
    Ok(())
}
pub(super) fn top(b: &mut FunctionBuilder<'_>, arena: Value, used: Value) -> Top {
    let one = c32(b, 1);
    let previous = b.ins().isub(used, one);
    let address = address(b, arena, previous);
    let leaf = load(b, types::I64, address, 0);
    let leaf = b.ins().ireduce(types::I32, leaf);
    let root = load(b, types::I64, address, 8);
    let ty = load(b, types::I32, address, 16);
    let offset = load(b, types::I32, address, 20);
    let index = load(b, types::I32, address, 24);
    let role = load(b, types::I32, address, 28);
    Top { previous, address, leaf, root, ty, offset, index, role }
}
