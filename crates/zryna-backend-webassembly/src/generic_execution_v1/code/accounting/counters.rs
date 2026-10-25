//! Fixed bounded global invariants, admission and result zeroing; cleanup consumes held credit.
use super::super::WORD;
use super::{Body, Diagnostic, I, canonical_failure};
use wasm_encoder::{BlockType, MemArg};
pub(super) fn zero(b: &mut Body<'_>, out: u32) -> Result<(), Diagnostic> {
    for offset in [0, 4, 8] {
        b.ops([I::LocalGet(out), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    Ok(())
}
pub(super) fn context(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([
        I::GlobalGet(10),
        I::I32Const(1_048_576),
        I::I32GtU,
        I::GlobalGet(8),
        I::GlobalGet(10),
        I::I32GtU,
        I::I32Or,
        I::GlobalGet(8),
        I::I32Const(1_048_576),
        I::I32GtU,
        I::I32Or,
        I::GlobalGet(11),
        I::I32Const(4_194_304),
        I::I32GtU,
        I::I32Or,
        I::GlobalGet(12),
        I::I32Const(4_194_304),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::GlobalGet(11),
        I::GlobalGet(12),
        I::I32Add,
        I::I32Const(4_194_304),
        I::I32GtU,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)
}
pub(super) fn funded(b: &mut Body<'_>, length: u32) -> Result<(), Diagnostic> {
    b.ops([
        I::GlobalGet(12),
        I::I32Eqz,
        I::LocalGet(length),
        I::I32Eqz,
        I::I32Eqz,
        I::GlobalGet(8),
        I::I32Eqz,
        I::I32And,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)
}
pub(super) fn admit(b: &mut Body<'_>, possible: u32, positive: u32) -> Result<(), Diagnostic> {
    b.ops([
        I::GlobalGet(11),
        I::GlobalGet(12),
        I::I32Add,
        I::I32Const(1),
        I::I32Add,
        I::LocalGet(possible),
        I::I32Add,
        I::I32Const(4_194_304),
        I::I32GtU,
        I::GlobalGet(10),
        I::LocalGet(positive),
        I::I32Add,
        I::I32Const(1_048_576),
        I::I32GtU,
        I::I32Or,
        I::GlobalGet(8),
        I::LocalGet(positive),
        I::I32Add,
        I::I32Const(1_048_576),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 3, 2)?;
    b.op(&I::End)?;
    b.ops([
        I::GlobalGet(11),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(11),
        I::GlobalGet(10),
        I::LocalGet(positive),
        I::I32Add,
        I::GlobalSet(10),
        I::GlobalGet(12),
        I::LocalGet(possible),
        I::I32Add,
        I::GlobalSet(12),
    ])
}
