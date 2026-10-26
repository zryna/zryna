//! Canonical container operations over the same verified private memory partition.
use super::{Body, Diagnostic, FunctionCode, I, Model, WORD, canonical_failure};
use crate::generic_execution_v1::model::containers as C;
use wasm_encoder::{BlockType, MemArg};
pub(super) mod allocation;
pub(super) mod counts;
mod reserve;
mod vector;
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    if m.container_count == 0 {
        return Ok(());
    }
    out.push(allocation::allocate(m)?);
    out.push(allocation::release(m)?);
    out.push(vector::allocate(m)?);
    out.push(vector::release(m)?);
    for kind in [C::STRONG_CLONE, C::DOWNGRADE, C::WEAK_CLONE] {
        out.push(counts::acquire(m, kind)?);
    }
    out.push(counts::begin(m)?);
    out.push(counts::finish(m)?);
    out.push(counts::release(m)?);
    if m.continuation {
        out.push(reserve::reserve(m)?);
        out.push(counts::upgrade(m)?);
    }
    Ok(())
}
fn number(n: u64) -> Result<I<'static>, Diagnostic> {
    Ok(I::I32Const(crate::generic_execution_v1::checked_u32(n)?.cast_signed()))
}

fn scratch(m: &Model<'_, '_, '_>, b: &mut Body<'_>, offset: u32) -> Result<(), Diagnostic> {
    b.op(&number(u64::from(m.memory.claims().scratch_start + offset))?)
}
fn zero(b: &mut Body<'_>, local: u32) -> Result<(), Diagnostic> {
    for offset in [0, 4, 8] {
        b.ops([I::LocalGet(local), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    Ok(())
}
fn valid(b: &mut Body<'_>, m: &Model<'_, '_, '_>, local: u32) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(local),
        number(u64::from(m.memory.claims().heap_start) + 16)?,
        I::I32LtU,
        I::LocalGet(local),
        I::GlobalGet(0),
        I::I32GeU,
        I::I32Or,
        I::LocalGet(local),
        I::I32Const(7),
        I::I32And,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(local),
        I::I32Const(16),
        I::I32Sub,
        I::I32Load(MemArg { offset: 8, ..WORD }),
        I::I32Const(1),
        I::I32Ne,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)
}
fn free(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(0),
        I::I32Const(16),
        I::I32Sub,
        I::I32Const(0),
        I::I32Store(MemArg { offset: 8, ..WORD }),
        I::GlobalGet(8),
        I::I32Const(1),
        I::I32Sub,
        I::GlobalSet(8),
    ])
}
