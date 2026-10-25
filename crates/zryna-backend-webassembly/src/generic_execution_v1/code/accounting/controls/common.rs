//! Fixed counter validation and typed failure paths; all instructions use the existing writer.
use super::{BlockType, Body, Diagnostic, I};
pub(super) fn fail(
    b: &mut Body<'_>,
    source: i32,
    status: i32,
    wide: bool,
) -> Result<(), Diagnostic> {
    b.ops([I::I32Const(source), I::GlobalSet(1)])?;
    b.op(&if wide { I::I64Const(i64::from(status) << 32) } else { I::I32Const(status) })?;
    b.op(&I::Return)
}
pub(super) fn context(b: &mut Body<'_>, wide: bool) -> Result<(), Diagnostic> {
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
    fail(b, 255, 255, wide)?;
    b.ops([
        I::End,
        I::GlobalGet(11),
        I::GlobalGet(12),
        I::I32Add,
        I::I32Const(4_194_304),
        I::I32GtU,
        I::If(BlockType::Empty),
    ])?;
    fail(b, 255, 255, wide)?;
    b.op(&I::End)
}
pub(super) fn funded(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([
        I::GlobalGet(12),
        I::I32Eqz,
        I::GlobalGet(8),
        I::I32Eqz,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    fail(b, 255, 255, false)?;
    b.op(&I::End)
}
pub(super) fn admit(
    b: &mut Body<'_>,
    hold: u32,
    positive: u32,
    wide: bool,
) -> Result<(), Diagnostic> {
    b.ops([
        I::GlobalGet(11),
        I::GlobalGet(12),
        I::I32Add,
        I::I32Const(1),
        I::I32Add,
        I::LocalGet(hold),
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
    fail(b, 3, 2, wide)?;
    b.ops([
        I::End,
        I::GlobalGet(11),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(11),
        I::GlobalGet(10),
        I::LocalGet(positive),
        I::I32Add,
        I::GlobalSet(10),
        I::GlobalGet(12),
        I::LocalGet(hold),
        I::I32Add,
        I::GlobalSet(12),
    ])
}
pub(super) fn refund(b: &mut Body<'_>, hold: u32) -> Result<(), Diagnostic> {
    b.ops([I::GlobalGet(12), I::LocalGet(hold), I::I32Sub, I::GlobalSet(12)])
}
