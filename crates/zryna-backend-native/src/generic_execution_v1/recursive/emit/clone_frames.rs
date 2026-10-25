//! Clone-only suffix frames; role/type packing is bounded by the existing sealed type census.
use super::util::{self as u, c32, c64, cmp, load, store, trap};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
#[derive(Clone, Copy)]
pub(super) struct Saved {
    pub source: Value,
    pub destination: Value,
    pub ty: Value,
    pub offset: Value,
    pub index: Value,
    pub vector: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Top {
    pub previous: Value,
    pub address: Value,
    pub source: Value,
    pub destination: Value,
    pub ty: Value,
    pub offset: Value,
    pub index: Value,
    pub vector: Value,
}
fn address(builder: &mut FunctionBuilder<'_>, arena: Value, index: Value) -> Value {
    let wide = builder.ins().uextend(types::I64, index);
    let width = c64(builder, 32);
    let bytes = builder.ins().imul(wide, width);
    let first = u::add(builder, arena, 16);
    builder.ins().iadd(first, bytes)
}
pub(super) fn push(
    builder: &mut FunctionBuilder<'_>,
    arena: Value,
    saved: Saved,
) -> Result<(), Diagnostic> {
    let used = load(builder, types::I32, arena, 0);
    let cap = c32(
        builder,
        u32::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| invariant())?,
    );
    let full = cmp(builder, IntCC::UnsignedGreaterThanOrEqual, used, cap);
    trap(builder, full);
    let frame = address(builder, arena, used);
    store(builder, frame, 0, saved.source);
    store(builder, frame, 8, saved.destination);
    store(builder, frame, 16, saved.ty);
    store(builder, frame, 20, saved.offset);
    store(builder, frame, 24, saved.index);
    let packed =
        saved.vector.checked_mul(4).and_then(|n| n.checked_add(2)).ok_or_else(invariant)?;
    let packed = c32(builder, packed);
    store(builder, frame, 28, packed);
    let one = c32(builder, 1);
    let next = builder.ins().iadd(used, one);
    store(builder, arena, 0, next);
    Ok(())
}
pub(super) fn top(builder: &mut FunctionBuilder<'_>, arena: Value, used: Value) -> Top {
    let one = c32(builder, 1);
    let previous = builder.ins().isub(used, one);
    let frame = address(builder, arena, previous);
    let source = load(builder, types::I64, frame, 0);
    let destination = load(builder, types::I64, frame, 8);
    let ty = load(builder, types::I32, frame, 16);
    let offset = load(builder, types::I32, frame, 20);
    let index = load(builder, types::I32, frame, 24);
    let packed = load(builder, types::I32, frame, 28);
    let mask = c32(builder, 3);
    let role = builder.ins().band(packed, mask);
    let two = c32(builder, 2);
    let wrong = cmp(builder, IntCC::NotEqual, role, two);
    trap(builder, wrong);
    let shift = c32(builder, 2);
    let vector = builder.ins().ushr(packed, shift);
    Top { previous, address: frame, source, destination, ty, offset, index, vector }
}
