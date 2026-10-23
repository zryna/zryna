//! Canonical Allocate i64 low-pointer/high-status result and exact Release arguments.
use super::{
    BlockType, Body, Diagnostic, FunctionCode, I, MemArg, Model, WORD, canonical_failure, free,
    number, valid,
};
pub(super) fn allocate(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 3)?;
    b.ops([
        I::LocalGet(1),
        I::I32Eqz,
        I::LocalGet(1),
        I::I32Const(8),
        I::I32GtU,
        I::I32Or,
        I::LocalGet(1),
        I::LocalGet(1),
        I::I32Const(1),
        I::I32Sub,
        I::I32And,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    fail(&mut b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(0),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::I64Const(0),
        I::Return,
        I::End,
        I::GlobalGet(5),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(5),
    ])?;
    if m.observed() {
        b.ops([
            I::GlobalGet(3),
            I::I32Const(2),
            I::I32Eq,
            I::GlobalGet(5),
            I::GlobalGet(4),
            I::I32Eq,
            I::I32And,
            I::If(BlockType::Empty),
        ])?;
        fail(&mut b, 2, 1)?;
        b.op(&I::End)?;
    }
    b.ops([
        I::LocalGet(0),
        number(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16)?,
        I::I32GtU,
        I::If(BlockType::Empty),
    ])?;
    fail(&mut b, 3, 2)?;
    b.op(&I::End)?;
    b.ops([
        I::GlobalGet(0),
        I::LocalTee(2),
        I::I32Const(16),
        I::I32Add,
        I::LocalTee(3),
        I::LocalGet(0),
        I::I32Add,
        I::I32Const(7),
        I::I32Add,
        I::I32Const(-8),
        I::I32And,
        I::LocalTee(4),
        I::LocalGet(2),
        I::I32LtU,
        I::LocalGet(4),
        number(u64::from(crate::generic_execution_v1::memory::MEMORY_BYTES))?,
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    fail(&mut b, 2, 1)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(4),
        I::GlobalSet(0),
        I::GlobalGet(7),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(7),
        I::GlobalGet(8),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(8),
    ])?;
    for (offset, op) in
        [(0, I::GlobalGet(7)), (4, I::LocalGet(0)), (8, I::I32Const(1)), (12, I::LocalGet(1))]
    {
        b.ops([I::LocalGet(2), op, I::I32Store(MemArg { offset, ..WORD })])?;
    }
    b.ops([I::LocalGet(3), I::I64ExtendI32U])?;
    let mut code = FunctionCode::new(2, 1, 3, b)?;
    code.result64 = true;
    Ok(code)
}
fn fail(b: &mut Body<'_>, trap: i32, status: i64) -> Result<(), Diagnostic> {
    b.ops([I::I32Const(trap), I::GlobalSet(1), I::I64Const(status << 32), I::Return])
}
pub(super) fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    b.ops([
        I::LocalGet(0),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(1),
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(&mut b, 255, 255)?;
    b.ops([I::End, I::I32Const(0), I::Return, I::End])?;
    valid(&mut b, m, 0)?;
    for (offset, argument) in [(4, 1), (12, 2)] {
        b.ops([
            I::LocalGet(0),
            I::I32Const(16),
            I::I32Sub,
            I::I32Load(MemArg { offset, ..WORD }),
            I::LocalGet(argument),
            I::I32Ne,
            I::If(BlockType::Empty),
        ])?;
        canonical_failure(&mut b, 255, 255)?;
        b.op(&I::End)?;
    }
    free(&mut b)?;
    b.op(&I::I32Const(0))?;
    FunctionCode::new(3, 1, 0, b)
}
