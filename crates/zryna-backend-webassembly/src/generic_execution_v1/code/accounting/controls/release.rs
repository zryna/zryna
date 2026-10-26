//! Mandatory funded release caches pre-free predicates and verifies kernel-owned L, never decrements it twice.
use super::{BlockType, Body, C, Diagnostic, FunctionCode, I, Model, WORD, common};
pub(super) fn emit(m: &Model<'_, '_, '_>, role: u32) -> Result<FunctionCode, Diagnostic> {
    let parameters = if role == C::STRONG_FINISH { 1 } else { 2 };
    let predicate = parameters;
    let old_live = parameters + 1;
    let status = parameters + 2;
    let mut b = Body::new(m, 3)?;
    if parameters == 2 {
        b.ops([I::LocalGet(1), I::I32Const(0), I::I32Store(WORD)])?;
    }
    common::context(&mut b, false)?;
    super::super::super::containers::counts::control(&mut b, m, role == C::STRONG_FINISH)?;
    common::funded(&mut b)?;
    validity(&mut b, role)?;
    if role == C::STRONG_BEGIN {
        b.load(0, 0)?;
        b.ops([I::I32Const(1), I::I32Eq])?;
    } else {
        b.load(0, 4)?;
        b.ops([I::I32Const(1), I::I32Eq])?;
        if role == C::WEAK_RELEASE {
            b.load(0, 0)?;
            b.ops([I::I32Eqz, I::I32And])?;
        }
    }
    b.ops([
        I::LocalSet(predicate),
        I::GlobalGet(8),
        I::LocalSet(old_live),
        I::GlobalGet(12),
        I::I32Const(1),
        I::I32Sub,
        I::GlobalSet(12),
        I::GlobalGet(11),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(11),
        I::LocalGet(0),
    ])?;
    if parameters == 2 {
        b.op(&I::LocalGet(1))?;
    }
    b.ops([
        I::Call(m.container_base + role),
        I::LocalSet(status),
        I::LocalGet(status),
        I::If(BlockType::Empty),
    ])?;
    common::fail(&mut b, 255, 255, false)?;
    b.op(&I::End)?;
    if parameters == 2 {
        b.load(1, 0)?;
        b.ops([I::LocalGet(predicate), I::I32Ne, I::If(BlockType::Empty)])?;
        common::fail(&mut b, 255, 255, false)?;
        b.op(&I::End)?;
    }
    b.ops([I::GlobalGet(8), I::LocalGet(old_live)])?;
    if role != C::STRONG_BEGIN {
        b.ops([I::LocalGet(predicate), I::I32Sub])?;
    }
    b.ops([I::I32Ne, I::If(BlockType::Empty)])?;
    common::fail(&mut b, 255, 255, false)?;
    b.ops([I::End, I::I32Const(0)])?;
    FunctionCode::new(parameters, 1, 3, b)
}
fn validity(b: &mut Body<'_>, role: u32) -> Result<(), Diagnostic> {
    if role == C::STRONG_BEGIN {
        b.load(0, 0)?;
        b.op(&I::I32Eqz)?;
    } else if role == C::STRONG_FINISH {
        b.load(0, 0)?;
        b.load(0, 4)?;
        b.ops([I::I32Eqz, I::I32Or])?;
    } else {
        b.load(0, 4)?;
        b.load(0, 0)?;
        b.ops([I::I32Eqz, I::I32Eqz, I::I32LeU])?;
    }
    b.op(&I::If(BlockType::Empty))?;
    common::fail(b, 255, 255, false)?;
    b.op(&I::End)
}
