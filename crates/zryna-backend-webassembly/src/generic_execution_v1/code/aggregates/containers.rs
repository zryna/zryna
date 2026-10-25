//! Source Clone frontier drives element cloning; only completed destination prefix is rolled back.
use super::{BlockType, Body, Diagnostic, I, MemArg, Model, WORD, invariant, return_if_failed};
use crate::generic_execution_v1::model::containers as C;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
fn number(n: u64) -> Result<I<'static>, Diagnostic> {
    Ok(I::I32Const(crate::generic_execution_v1::checked_u32(n)?.cast_signed()))
}
fn check(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([I::If(BlockType::Empty), I::Unreachable, I::End])
}
pub(super) fn clone(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    ty: u32,
    kind: CloneNodeKind,
) -> Result<(), Diagnostic> {
    if kind == CloneNodeKind::VecEach {
        return vector(m, b, ty);
    }
    let operation = match kind {
        CloneNodeKind::SharedCount => C::STRONG_CLONE,
        CloneNodeKind::WeakCount => C::WEAK_CLONE,
        _ => return Err(invariant()),
    };
    b.load(0, 0)?;
    b.op(&I::Call(m.container(operation)))?;
    return_if_failed(b)?;
    b.op(&I::LocalGet(1))?;
    b.load(0, 0)?;
    b.op(&I::I32Store(WORD))
}
fn vector(m: &Model<'_, '_, '_>, b: &mut Body<'_>, ty: u32) -> Result<(), Diagnostic> {
    let meta = m.element(ty)?;
    let child = meta.element().index();
    let stride = number(meta.stride())?;
    b.op(&number(u64::from(child))?)?;
    b.load(0, 4)?;
    b.ops([I::LocalGet(1), I::Call(m.container(C::VEC_ALLOCATE))])?;
    return_if_failed(b)?;
    b.ops([I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(3)])?;
    b.load(0, 4)?;
    b.ops([I::I32GeU, I::BrIf(1)])?;
    for local in [0, 1] {
        b.load(local, 0)?;
        b.ops([I::LocalGet(3), stride.clone(), I::I32Mul, I::I32Add])?;
    }
    b.ops([
        I::Call(m.clone_index(child)),
        I::LocalTee(2),
        I::If(BlockType::Empty),
        I::Block(BlockType::Empty),
        I::Loop(BlockType::Empty),
        I::LocalGet(3),
        I::I32Eqz,
        I::BrIf(1),
        I::LocalGet(3),
        I::I32Const(1),
        I::I32Sub,
        I::LocalSet(3),
    ])?;
    b.load(1, 0)?;
    b.ops([
        I::LocalGet(3),
        stride,
        I::I32Mul,
        I::I32Add,
        I::Call(m.drop_index(child)),
        I::Br(0),
        I::End,
        I::End,
    ])?;
    release_vec(m, b, ty, 1)?;
    b.ops([
        I::LocalGet(2),
        I::Return,
        I::End,
        I::LocalGet(3),
        I::I32Const(1),
        I::I32Add,
        I::LocalSet(3),
        I::LocalGet(1),
        I::LocalGet(3),
        I::I32Store(MemArg { offset: 4, ..WORD }),
        I::Br(0),
        I::End,
        I::End,
    ])
}
fn release_vec(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    ty: u32,
    local: u32,
) -> Result<(), Diagnostic> {
    b.op(&number(u64::from(m.element(ty)?.element().index()))?)?;
    for offset in [0, 4, 8] {
        b.load(local, offset)?;
    }
    b.op(&I::Call(m.container(C::VEC_RELEASE)))?;
    check(b)?;
    zero(b, local)
}
fn zero(b: &mut Body<'_>, local: u32) -> Result<(), Diagnostic> {
    for offset in [0, 4, 8] {
        b.ops([I::LocalGet(local), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    Ok(())
}
pub(super) fn drop(m: &Model<'_, '_, '_>, b: &mut Body<'_>, ty: u32) -> Result<(), Diagnostic> {
    match m.types[ty as usize].category() {
        TypeCategory::Vec => {
            let meta = m.element(ty)?;
            let child = meta.element().index();
            b.load(0, 4)?;
            b.ops([
                I::LocalSet(1),
                I::Block(BlockType::Empty),
                I::Loop(BlockType::Empty),
                I::LocalGet(1),
                I::I32Eqz,
                I::BrIf(1),
                I::LocalGet(1),
                I::I32Const(1),
                I::I32Sub,
                I::LocalTee(1),
                number(meta.stride())?,
                I::I32Mul,
            ])?;
            b.load(0, 0)?;
            b.ops([I::I32Add, I::Call(m.drop_index(child)), I::Br(0), I::End, I::End])?;
            release_vec(m, b, ty, 0)?;
        }
        TypeCategory::Shared => {
            b.load(0, 0)?;
            b.op(&number(u64::from(m.memory.claims().scratch_start))?)?;
            b.op(&I::Call(m.container(C::STRONG_BEGIN)))?;
            check(b)?;
            b.ops([
                number(u64::from(m.memory.claims().scratch_start))?,
                I::I32Load(WORD),
                I::LocalSet(1),
                I::LocalGet(1),
                I::If(BlockType::Empty),
            ])?;
            b.load(0, 0)?;
            let meta = m.control(ty)?;
            b.ops([
                number(meta.payload_offset())?,
                I::I32Add,
                I::Call(m.drop_index(meta.payload().index())),
            ])?;
            b.load(0, 0)?;
            b.op(&I::Call(m.container(C::STRONG_FINISH)))?;
            check(b)?;
            b.op(&I::End)?;
            b.ops([I::LocalGet(0), I::I32Const(0), I::I32Store(WORD)])?;
        }
        TypeCategory::Weak => {
            b.load(0, 0)?;
            b.op(&number(u64::from(m.memory.claims().scratch_start))?)?;
            b.op(&I::Call(m.container(C::WEAK_RELEASE)))?;
            check(b)?;
            b.ops([I::LocalGet(0), I::I32Const(0), I::I32Store(WORD)])?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
