//! Independent actual root lease recipe; every guard/store is consumed in exact order.
use super::cursor::Cursor;
use cranelift_codegen::ir::{FuncRef, Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
pub(super) fn begin(c: &mut Cursor<'_>, getter: FuncRef, ctx: Value) -> Result<Value, Diagnostic> {
    let base = c.call(getter, &[ctx], types::I64)?;
    let used = c.load(types::I32, base, 0)?;
    let busy = c.load(types::I32, base, 4)?;
    let owner = c.load(types::I64, base, 8)?;
    let zero = c.constant(types::I32, 0)?;
    let used = c.compare(IntCC::NotEqual, [used, zero])?;
    let busy = c.compare(IntCC::NotEqual, [busy, zero])?;
    let null = c.constant(types::I64, 0)?;
    let owner = c.compare(IntCC::NotEqual, [owner, null])?;
    let bad = c.binary(O::Bor, types::I8, [used, busy])?;
    let bad = c.binary(O::Bor, types::I8, [bad, owner])?;
    c.trap(bad)?;
    let one = c.constant(types::I32, 1)?;
    c.store(base, 4, one)?;
    c.store(base, 8, ctx)?;
    Ok(base)
}
pub(super) fn completed(c: &mut Cursor<'_>, base: Value, ctx: Value) -> Result<(), Diagnostic> {
    let used = c.load(types::I32, base, 0)?;
    let busy = c.load(types::I32, base, 4)?;
    let owner = c.load(types::I64, base, 8)?;
    let zero = c.constant(types::I32, 0)?;
    let used = c.compare(IntCC::NotEqual, [used, zero])?;
    let one = c.constant(types::I32, 1)?;
    let busy = c.compare(IntCC::NotEqual, [busy, one])?;
    let owner = c.compare(IntCC::NotEqual, [owner, ctx])?;
    let bad = c.binary(O::Bor, types::I8, [used, busy])?;
    let bad = c.binary(O::Bor, types::I8, [bad, owner])?;
    c.trap(bad)
}
pub(super) fn end(c: &mut Cursor<'_>, base: Value) -> Result<(), Diagnostic> {
    let null = c.constant(types::I64, 0)?;
    c.store(base, 8, null)?;
    let zero = c.constant(types::I32, 0)?;
    c.store(base, 4, zero)
}
