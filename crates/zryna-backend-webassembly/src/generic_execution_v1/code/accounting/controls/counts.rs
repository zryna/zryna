//! Fresh actual count fields distinguish potential owner H1 from known REFCOUNT3/EXPIRED5 H0.
use super::{BlockType, Body, C, Diagnostic, FunctionCode, I, Model, common};
pub(super) fn emit(m: &Model<'_, '_, '_>, role: u32) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 4)?;
    common::context(&mut b, false)?;
    super::super::super::containers::counts::control(&mut b, m, false)?;
    common::funded(&mut b)?;
    let field = if role == C::DOWNGRADE || role == C::WEAK_CLONE { 4 } else { 0 };
    let required = if role == C::WEAK_CLONE || role == C::WEAK_UPGRADE { 4 } else { 0 };
    b.load(0, required)?;
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    common::fail(&mut b, 255, 255, false)?;
    b.op(&I::End)?;
    b.load(0, field)?;
    b.ops([I::I32Const(-1), I::I32Ne])?;
    if role == C::WEAK_UPGRADE {
        b.load(0, 0)?;
        b.ops([I::I32Eqz, I::I32Eqz, I::I32And])?;
    }
    b.ops([I::LocalSet(1), I::I32Const(0), I::LocalSet(2), I::GlobalGet(8), I::LocalSet(3)])?;
    common::admit(&mut b, 1, 2, false)?;
    b.ops([
        I::LocalGet(0),
        I::Call(m.container_base + role),
        I::LocalSet(4),
        I::GlobalGet(8),
        I::LocalGet(3),
        I::I32Ne,
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, false)?;
    b.op(&I::End)?;
    b.ops([I::LocalGet(4), I::If(BlockType::Empty)])?;
    common::refund(&mut b, 1)?;
    for (index, status) in [3, 255].into_iter().enumerate() {
        b.ops([I::LocalGet(4), I::I32Const(status), I::I32Eq])?;
        if index > 0 {
            b.op(&I::I32Or)?;
        }
    }
    if role == C::WEAK_UPGRADE {
        b.ops([I::LocalGet(4), I::I32Const(5), I::I32Eq, I::I32Or])?;
    }
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    common::fail(&mut b, 255, 255, false)?;
    b.ops([
        I::End,
        I::LocalGet(4),
        I::Return,
        I::End,
        I::LocalGet(1),
        I::I32Eqz,
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, false)?;
    b.ops([I::End, I::I32Const(0)])?;
    FunctionCode::new(1, 1, 4, b)
}
