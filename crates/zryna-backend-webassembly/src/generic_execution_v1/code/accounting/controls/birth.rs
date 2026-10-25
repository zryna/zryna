//! Packed I64 birth result uses existing scratch; canonical allocation alone owns physical L8.
use super::{BlockType, Body, C, Diagnostic, FunctionCode, I, MemArg, Model, WORD, common};
pub(super) fn emit(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 6)?;
    let scratch = m.memory.claims().scratch_start.cast_signed();
    b.ops([
        I::I32Const(scratch),
        I::I64Const(0),
        I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
    ])?;
    common::context(&mut b, true)?;
    classify(&mut b)?;
    common::admit(&mut b, 2, 3, true)?;
    b.ops([
        I::I32Const(scratch),
        I::LocalGet(0),
        I::LocalGet(1),
        I::Call(m.container_base + C::ALLOCATE_RAW),
        I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
        I::I32Const(scratch),
        I::I32Load(MemArg { offset: 4, ..WORD }),
        I::LocalSet(4),
        I::I32Const(scratch),
        I::I32Load(WORD),
        I::LocalSet(5),
        I::LocalGet(4),
        I::If(BlockType::Empty),
    ])?;
    common::refund(&mut b, 2)?;
    b.ops([
        I::LocalGet(5),
        I::GlobalGet(8),
        I::LocalGet(6),
        I::I32Ne,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, true)?;
    b.op(&I::End)?;
    for (index, status) in [1, 2, 255].into_iter().enumerate() {
        b.ops([I::LocalGet(4), I::I32Const(status), I::I32Eq])?;
        if index > 0 {
            b.op(&I::I32Or)?;
        }
    }
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    common::fail(&mut b, 255, 255, true)?;
    b.ops([
        I::End,
        I::LocalGet(4),
        I::I64ExtendI32U,
        I::I64Const(32),
        I::I64Shl,
        I::Return,
        I::End,
    ])?;
    // Zero-size raw Allocate legitimately returns zero with no owner; every positive success was reserved.
    b.ops([
        I::LocalGet(0),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(5),
        I::GlobalGet(8),
        I::LocalGet(6),
        I::I32Ne,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, true)?;
    b.ops([
        I::End,
        I::I64Const(0),
        I::Return,
        I::End,
        I::LocalGet(3),
        I::I32Eqz,
        I::LocalGet(5),
        I::I32Eqz,
        I::I32Or,
        I::GlobalGet(8),
        I::LocalGet(6),
        I::I32Const(1),
        I::I32Add,
        I::I32Ne,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, true)?;
    b.ops([I::End, I::LocalGet(5), I::I64ExtendI32U])?;
    let mut result = FunctionCode::new(2, 1, 6, b)?;
    result.result64 = true;
    Ok(result)
}

fn classify(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    // A valid positive allocation can create a control owner; known capacity/invalid alignment cannot.
    b.ops([
        I::LocalGet(0),
        I::I32Eqz,
        I::I32Eqz,
        I::LocalGet(0),
        I::I32Const(2_147_483_647),
        I::I32LeU,
        I::I32And,
        I::LocalGet(1),
        I::I32Eqz,
        I::I32Eqz,
        I::I32And,
        I::LocalGet(1),
        I::I32Const(8),
        I::I32LeU,
        I::I32And,
        I::LocalGet(1),
        I::LocalGet(1),
        I::I32Const(1),
        I::I32Sub,
        I::I32And,
        I::I32Eqz,
        I::I32And,
        I::LocalTee(3),
        I::I32Const(2),
        I::I32Mul,
        I::LocalSet(2),
        I::GlobalGet(8),
        I::LocalSet(6),
    ])
}
