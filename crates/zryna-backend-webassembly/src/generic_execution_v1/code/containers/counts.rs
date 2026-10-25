//! Count-only terminals; last strong drops payload before the implicit weak is released.
use super::{
    BlockType, Body, C, Diagnostic, FunctionCode, I, MemArg, Model, WORD, canonical_failure, free,
    valid,
};
fn header(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([I::LocalGet(0), I::I32Const(16), I::I32Sub])
}
fn load_flags(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    header(b)?;
    b.op(&I::I32Load(MemArg { offset: 12, ..WORD }))
}
pub(in crate::generic_execution_v1::code) fn control(
    b: &mut Body<'_>,
    m: &Model<'_, '_, '_>,
    pending: bool,
) -> Result<(), Diagnostic> {
    valid(b, m, 0)?;
    header(b)?;
    b.ops([I::I32Load(MemArg { offset: 4, ..WORD }), I::I32Const(8), I::I32LtU])?;
    load_flags(b)?;
    b.ops([I::I32Const(256), I::I32And])?;
    if pending {
        b.op(&I::I32Eqz)?;
    }
    b.ops([I::I32Or, I::If(BlockType::Empty)])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)
}
pub(super) fn acquire(m: &Model<'_, '_, '_>, kind: u32) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    control(&mut b, m, false)?;
    let field = if kind == C::STRONG_CLONE { 0 } else { 4 };
    b.load(0, if kind == C::WEAK_CLONE { 4 } else { 0 })?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    b.load(0, field)?;
    b.ops([I::I32Const(-1), I::I32Eq, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 4, 3)?;
    b.op(&I::End)?;
    b.op(&I::LocalGet(0))?;
    b.load(0, field)?;
    b.ops([
        I::I32Const(1),
        I::I32Add,
        I::I32Store(MemArg { offset: u64::from(field), ..WORD }),
        I::I32Const(0),
    ])?;
    FunctionCode::new(1, 1, 0, b)
}
pub(super) fn begin(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    b.ops([I::LocalGet(1), I::I32Const(0), I::I32Store(WORD)])?;
    control(&mut b, m, false)?;
    b.load(0, 0)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    b.op(&I::LocalGet(0))?;
    b.load(0, 0)?;
    b.ops([I::I32Const(1), I::I32Sub, I::I32Store(WORD)])?;
    b.load(0, 0)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    header(&mut b)?;
    load_flags(&mut b)?;
    b.ops([
        I::I32Const(256),
        I::I32Or,
        I::I32Store(MemArg { offset: 12, ..WORD }),
        I::LocalGet(1),
        I::I32Const(1),
        I::I32Store(WORD),
        I::End,
        I::I32Const(0),
    ])?;
    FunctionCode::new(2, 1, 0, b)
}
pub(super) fn finish(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    control(&mut b, m, true)?;
    b.load(0, 0)?;
    b.load(0, 4)?;
    b.ops([I::I32Eqz, I::I32Or, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    header(&mut b)?;
    load_flags(&mut b)?;
    b.ops([I::I32Const(255), I::I32And, I::I32Store(MemArg { offset: 12, ..WORD })])?;
    b.op(&I::LocalGet(0))?;
    b.load(0, 4)?;
    b.ops([I::I32Const(1), I::I32Sub, I::I32Store(MemArg { offset: 4, ..WORD })])?;
    b.load(0, 4)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    free(&mut b)?;
    b.ops([I::End, I::I32Const(0)])?;
    FunctionCode::new(1, 1, 0, b)
}
pub(super) fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    b.ops([I::LocalGet(1), I::I32Const(0), I::I32Store(WORD)])?;
    control(&mut b, m, false)?;
    b.load(0, 4)?;
    b.load(0, 0)?;
    b.ops([I::I32Eqz, I::I32Eqz, I::I32LeU, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    b.op(&I::LocalGet(0))?;
    b.load(0, 4)?;
    b.ops([I::I32Const(1), I::I32Sub, I::I32Store(MemArg { offset: 4, ..WORD })])?;
    b.load(0, 0)?;
    b.load(0, 4)?;
    b.ops([I::I32Or, I::I32Eqz, I::If(BlockType::Empty)])?;
    free(&mut b)?;
    b.ops([I::LocalGet(1), I::I32Const(1), I::I32Store(WORD), I::End, I::I32Const(0)])?;
    FunctionCode::new(2, 1, 0, b)
}

/// Expiration is an unchanged-count branch result, not a controlled failure trap.
pub(super) fn upgrade(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    control(&mut b, m, false)?;
    b.load(0, 4)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    b.load(0, 0)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty), I::I32Const(5), I::Return, I::End])?;
    b.load(0, 0)?;
    b.ops([I::I32Const(-1), I::I32Eq, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 4, 3)?;
    b.op(&I::End)?;
    b.op(&I::LocalGet(0))?;
    b.load(0, 0)?;
    b.ops([I::I32Const(1), I::I32Add, I::I32Store(WORD), I::I32Const(0)])?;
    FunctionCode::new(1, 1, 0, b)
}
