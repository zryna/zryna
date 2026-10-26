//! Independent exact fresh postsource counter checks; no quota reset/denial branch.
use super::Cursor;
use cranelift_codegen::ir::{Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
pub(super) fn check(cursor: &mut Cursor<'_>, ctx: Value) -> Result<(), Diagnostic> {
    let attempt_count = cursor.load(types::I64, ctx, 0)?;
    let live_count = cursor.load(types::I64, ctx, 8)?;
    let spent_count = cursor.load(types::I64, ctx, 16)?;
    let held_count = cursor.load(types::I64, ctx, 24)?;
    let max_a = cursor.constant(
        types::I64,
        zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS.cast_signed(),
    )?;
    let attempts = cursor.compare(IntCC::UnsignedGreaterThan, [attempt_count, max_a])?;
    let live = cursor.compare(IntCC::UnsignedGreaterThan, [live_count, attempt_count])?;
    let max_s = cursor
        .constant(types::I64, zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS.cast_signed())?;
    let spent = cursor.compare(IntCC::UnsignedGreaterThan, [spent_count, max_s])?;
    let max_h = cursor
        .constant(types::I64, zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS.cast_signed())?;
    let held = cursor.compare(IntCC::UnsignedGreaterThan, [held_count, max_h])?;
    let left = cursor.binary(O::Bor, types::I8, [attempts, live])?;
    let right = cursor.binary(O::Bor, types::I8, [spent, held])?;
    let bad = cursor.binary(O::Bor, types::I8, [left, right])?;
    cursor.trap(bad)?;
    let total = cursor.binary(O::Iadd, types::I64, [spent_count, held_count])?;
    let max = cursor
        .constant(types::I64, zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS.cast_signed())?;
    let bad = cursor.compare(IntCC::UnsignedGreaterThan, [total, max])?;
    cursor.trap(bad)?;
    let left = cursor.binary(O::Bor, types::I64, [live_count, held_count])?;
    cursor.trap(left)
}
