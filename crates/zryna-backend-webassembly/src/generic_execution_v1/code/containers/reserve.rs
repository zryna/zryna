//! Failure-atomic canonical geometric reserve through a four-argument private descriptor adapter.
use super::{
    BlockType, Body, C, Diagnostic, FunctionCode, I, MemArg, Model, WORD, canonical_failure,
    number, scratch, valid, zero,
};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1::ElementLayout;
pub(super) fn reserve(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 5)?;
    for (offset, local) in [(0, 4), (4, 5), (8, 6)] {
        b.load(1, offset)?;
        b.op(&I::LocalSet(local))?;
    }
    zero(&mut b, 3)?;
    b.ops([I::LocalGet(5), I::LocalGet(6), I::I32GtU, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|row| row.target() == StorageTarget::Linear32V1) {
        element(m, &mut b, row)?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(4, 1, 5, b)
}

fn element(m: &Model<'_, '_, '_>, b: &mut Body<'_>, row: &ElementLayout) -> Result<(), Diagnostic> {
    let ceiling = zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
        .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride());
    b.ops([
        I::LocalGet(0),
        number(u64::from(row.element().index()))?,
        I::I32Eq,
        I::If(BlockType::Empty),
    ])?;
    for local in [6, 2] {
        b.ops([I::LocalGet(local), number(ceiling)?, I::I32GtU, I::If(BlockType::Empty)])?;
        if local == 6 {
            canonical_failure(b, 255, 255)?;
        } else {
            canonical_failure(b, 3, 2)?;
        }
        b.op(&I::End)?;
    }
    old_record(m, b, row)?;
    candidate(b, ceiling)?;
    publish(m, b, row)
}
fn old_record(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    row: &ElementLayout,
) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(6),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(4),
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::Else])?;
    valid(b, m, 4)?;
    for (offset, local, factor) in [(4, 6, row.stride()), (12, u32::MAX, row.alignment())] {
        b.ops([I::LocalGet(4), I::I32Const(16), I::I32Sub, I::I32Load(MemArg { offset, ..WORD })])?;
        if local == u32::MAX {
            b.op(&number(factor)?)?;
        } else {
            b.ops([I::LocalGet(local), number(factor)?, I::I32Mul])?;
        }
        b.ops([I::I32Ne, I::If(BlockType::Empty)])?;
        canonical_failure(b, 255, 255)?;
        b.op(&I::End)?;
    }
    b.ops([
        I::End,
        I::LocalGet(2),
        I::LocalGet(6),
        I::I32LeU,
        I::If(BlockType::Empty),
        I::LocalGet(1),
        I::LocalGet(3),
        I::I32Const(12),
        I::Call(crate::generic_execution_v1::model::COPY),
        I::I32Const(0),
        I::Return,
        I::End,
    ])?;
    Ok(())
}
fn candidate(b: &mut Body<'_>, ceiling: u64) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(6),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::I32Const(4),
        I::LocalSet(7),
        I::Else,
        I::LocalGet(6),
        I::I32Const(2),
        I::I32Mul,
        I::LocalSet(7),
        I::End,
        I::Block(BlockType::Empty),
        I::Loop(BlockType::Empty),
        I::LocalGet(7),
        I::LocalGet(2),
        I::I32GeU,
        I::BrIf(1),
        I::LocalGet(7),
        number(ceiling / 2)?,
        I::I32GtU,
        I::If(BlockType::Empty),
        I::LocalGet(2),
        I::LocalSet(7),
        I::Br(2),
        I::End,
        I::LocalGet(7),
        I::I32Const(2),
        I::I32Mul,
        I::LocalSet(7),
        I::Br(0),
        I::End,
        I::End,
        I::LocalGet(7),
        number(ceiling)?,
        I::I32GtU,
        I::If(BlockType::Empty),
        I::LocalGet(2),
        I::LocalSet(7),
        I::End,
    ])?;
    Ok(())
}
fn publish(m: &Model<'_, '_, '_>, b: &mut Body<'_>, row: &ElementLayout) -> Result<(), Diagnostic> {
    scratch(m, b, 0)?;
    b.ops([
        I::LocalGet(7),
        number(row.stride())?,
        I::I32Mul,
        number(row.alignment())?,
        I::Call(m.container(C::ALLOCATE_RAW)),
        I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
    ])?;
    scratch(m, b, 4)?;
    b.ops([I::I32Load(WORD), I::If(BlockType::Empty)])?;
    scratch(m, b, 4)?;
    b.ops([I::I32Load(WORD), I::Return, I::End])?;
    scratch(m, b, 0)?;
    b.ops([
        I::I32Load(WORD),
        I::LocalSet(8),
        I::LocalGet(4),
        I::LocalGet(8),
        I::LocalGet(5),
        number(row.stride())?,
        I::I32Mul,
        I::Call(crate::generic_execution_v1::model::COPY),
        I::LocalGet(4),
        I::LocalGet(6),
        number(row.stride())?,
        I::I32Mul,
        number(row.alignment())?,
        I::Call(m.container(C::RELEASE_RAW)),
        I::If(BlockType::Empty),
        I::Unreachable,
        I::End,
    ])?;
    for (offset, local) in [(0, 8), (4, 5), (8, 7)] {
        b.ops([I::LocalGet(3), I::LocalGet(local), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    b.ops([I::I32Const(0), I::Return, I::End])?;
    Ok(())
}
