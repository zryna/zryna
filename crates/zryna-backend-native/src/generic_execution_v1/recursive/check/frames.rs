//! Independent complete preallocated frame grammar: bounds, widths, publication and restore.
use super::cursor::Cursor;
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{Opcode as O, Value, condcodes::IntCC, types};
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
fn address(c: &mut Cursor<'_>, arena: Value, index: Value) -> Result<Value, Diagnostic> {
    let wide = c.unary(O::Uextend, types::I64, index)?;
    let width = c.constant(types::I64, 32)?;
    let bytes = c.binary(O::Imul, types::I64, [wide, width])?;
    let header = c.constant(types::I64, 16)?;
    let first = c.binary(O::Iadd, types::I64, [arena, header])?;
    c.binary(O::Iadd, types::I64, [first, bytes])
}
pub(super) fn push(c: &mut Cursor<'_>, arena: Value, p: Pending) -> Result<(), Diagnostic> {
    let used = c.load(types::I32, arena, 0)?;
    let cap = c.constant(
        types::I32,
        i64::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| invariant())?,
    )?;
    let full = c.compare(IntCC::UnsignedGreaterThanOrEqual, [used, cap])?;
    c.trap(full)?;
    let destination = address(c, arena, used)?;
    let leaf = c.constant(types::I64, i64::from(p.leaf))?;
    c.store(destination, 0, leaf)?;
    c.store(destination, 8, p.root)?;
    c.store(destination, 16, p.ty)?;
    c.store(destination, 20, p.offset)?;
    c.store(destination, 24, p.index)?;
    let role = c.constant(types::I32, i64::from(p.role))?;
    c.store(destination, 28, role)?;
    let one = c.constant(types::I32, 1)?;
    let next = c.binary(O::Iadd, types::I32, [used, one])?;
    c.store(arena, 0, next)
}
pub(super) fn top(c: &mut Cursor<'_>, arena: Value, used: Value) -> Result<Top, Diagnostic> {
    let one = c.constant(types::I32, 1)?;
    let previous = c.binary(O::Isub, types::I32, [used, one])?;
    let address = address(c, arena, previous)?;
    let raw = c.load(types::I64, address, 0)?;
    let leaf = c.unary(O::Ireduce, types::I32, raw)?;
    let root = c.load(types::I64, address, 8)?;
    let ty = c.load(types::I32, address, 16)?;
    let offset = c.load(types::I32, address, 20)?;
    let index = c.load(types::I32, address, 24)?;
    let role = c.load(types::I32, address, 28)?;
    Ok(Top { previous, address, leaf, root, ty, offset, index, role })
}
