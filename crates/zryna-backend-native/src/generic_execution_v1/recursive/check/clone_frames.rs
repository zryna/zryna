//! Independent complete Clone-frame suffix recipe and exact packed role/type decoding.
use super::cursor::Cursor;
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{Opcode as O, Value, condcodes::IntCC, types};
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
fn address(reader: &mut Cursor<'_>, arena: Value, index: Value) -> Result<Value, Diagnostic> {
    let wide = reader.unary(O::Uextend, types::I64, index)?;
    let width = reader.constant(types::I64, 32)?;
    let bytes = reader.binary(O::Imul, types::I64, [wide, width])?;
    let header = reader.constant(types::I64, 16)?;
    let first = reader.binary(O::Iadd, types::I64, [arena, header])?;
    reader.binary(O::Iadd, types::I64, [first, bytes])
}
pub(super) fn push(reader: &mut Cursor<'_>, arena: Value, saved: Saved) -> Result<(), Diagnostic> {
    let used = reader.load(types::I32, arena, 0)?;
    let cap = reader.constant(
        types::I32,
        i64::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| invariant())?,
    )?;
    let full = reader.compare(IntCC::UnsignedGreaterThanOrEqual, [used, cap])?;
    reader.trap(full)?;
    let frame = address(reader, arena, used)?;
    reader.store(frame, 0, saved.source)?;
    reader.store(frame, 8, saved.destination)?;
    reader.store(frame, 16, saved.ty)?;
    reader.store(frame, 20, saved.offset)?;
    reader.store(frame, 24, saved.index)?;
    let packed =
        saved.vector.checked_mul(4).and_then(|n| n.checked_add(2)).ok_or_else(invariant)?;
    let word = reader.constant(types::I32, i64::from(packed))?;
    reader.store(frame, 28, word)?;
    let one = reader.constant(types::I32, 1)?;
    let next = reader.binary(O::Iadd, types::I32, [used, one])?;
    reader.store(arena, 0, next)
}
pub(super) fn top(reader: &mut Cursor<'_>, arena: Value, used: Value) -> Result<Top, Diagnostic> {
    let one = reader.constant(types::I32, 1)?;
    let previous = reader.binary(O::Isub, types::I32, [used, one])?;
    let frame = address(reader, arena, previous)?;
    let source = reader.load(types::I64, frame, 0)?;
    let destination = reader.load(types::I64, frame, 8)?;
    let ty = reader.load(types::I32, frame, 16)?;
    let offset = reader.load(types::I32, frame, 20)?;
    let index = reader.load(types::I32, frame, 24)?;
    let packed = reader.load(types::I32, frame, 28)?;
    let mask = reader.constant(types::I32, 3)?;
    let role = reader.binary(O::Band, types::I32, [packed, mask])?;
    let two = reader.constant(types::I32, 2)?;
    let wrong = reader.compare(IntCC::NotEqual, [role, two])?;
    reader.trap(wrong)?;
    let shift = reader.constant(types::I32, 2)?;
    let vector = reader.binary(O::Ushr, types::I32, [packed, shift])?;
    Ok(Top { previous, address: frame, source, destination, ty, offset, index, vector })
}
