//! Vec3 outer roles: storage debt1; raw allocator/release factoring has no extra logical debit.
use super::counters::{admit, context, funded, zero};
use super::{BlockType, Body, Diagnostic, FunctionCode, I, Model, canonical_failure};
use crate::generic_execution_v1::model::containers as C;
use zryna_layout::StorageTarget;
mod storage;
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    if m.has_accounted_vec() {
        out.push(allocate(m)?);
        out.push(release(m)?);
        if m.continuation {
            out.push(reserve(m)?);
        }
    }
    Ok(())
}
fn number(n: u64) -> Result<I<'static>, Diagnostic> {
    Ok(I::I32Const(crate::generic_execution_v1::checked_u32(n)?.cast_signed()))
}
fn row_start(b: &mut Body<'_>, id: u32) -> Result<(), Diagnostic> {
    b.ops([I::LocalGet(0), number(u64::from(id))?, I::I32Eq, I::If(BlockType::Empty)])
}
fn allocate(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 4)?;
    zero(&mut b, 2)?;
    context(&mut b)?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|row| row.target() == StorageTarget::Linear32V1) {
        row_start(&mut b, row.element().index())?;
        let ceiling = zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
            .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride());
        b.ops([
            I::LocalGet(1),
            number(ceiling)?,
            I::I32LeU,
            I::LocalSet(4),
            I::LocalGet(1),
            I::I32Eqz,
            I::I32Eqz,
            I::LocalGet(4),
            I::I32And,
            I::LocalSet(5),
        ])?;
        admit(&mut b, 4, 5)?;
        known_bound(&mut b, 4)?;
        b.ops([
            I::LocalGet(0),
            I::LocalGet(1),
            I::LocalGet(2),
            I::Call(m.container_base + C::VEC_ALLOCATE),
            I::LocalSet(6),
        ])?;
        complete(&mut b, 2, 6, 4)?;
        b.op(&I::End)?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(3, 1, 4, b)
}
fn reserve(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 7)?;
    for (offset, local) in [(8, 4), (0, 5), (4, 6)] {
        b.load(1, offset)?;
        b.op(&I::LocalSet(local))?;
    }
    zero(&mut b, 3)?;
    context(&mut b)?;
    funded(&mut b, 4)?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|row| row.target() == StorageTarget::Linear32V1) {
        row_start(&mut b, row.element().index())?;
        storage::valid(m, &mut b, row, 5, 6, 4)?;
        let ceiling = zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
            .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride());
        b.ops([
            I::LocalGet(2),
            number(ceiling)?,
            I::I32LeU,
            I::LocalSet(7),
            I::LocalGet(2),
            I::LocalGet(4),
            I::I32GtU,
            I::LocalGet(7),
            I::I32And,
            I::LocalSet(8),
        ])?;
        // Local9 is canonical zero-initialized: reserve transfers existing storage debt.
        admit(&mut b, 9, 8)?;
        known_bound(&mut b, 7)?;
        b.ops([
            I::LocalGet(0),
            I::LocalGet(1),
            I::LocalGet(2),
            I::LocalGet(3),
            I::Call(m.container_base + C::VEC_RESERVE),
            I::LocalSet(10),
        ])?;
        complete(&mut b, 3, 10, 9)?;
        b.op(&I::End)?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(4, 1, 7, b)
}
fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 1)?;
    context(&mut b)?;
    funded(&mut b, 3)?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|row| row.target() == StorageTarget::Linear32V1) {
        row_start(&mut b, row.element().index())?;
        storage::valid(m, &mut b, row, 1, 2, 3)?;
        b.ops([
            I::GlobalGet(12),
            I::I32Const(1),
            I::I32Sub,
            I::GlobalSet(12),
            I::GlobalGet(11),
            I::I32Const(1),
            I::I32Add,
            I::GlobalSet(11),
            I::LocalGet(0),
            I::LocalGet(1),
            I::LocalGet(2),
            I::LocalGet(3),
            I::Call(m.container_base + C::VEC_RELEASE),
            I::LocalSet(4),
            I::LocalGet(4),
            I::If(BlockType::Empty),
        ])?;
        canonical_failure(&mut b, 255, 255)?;
        b.ops([I::End, I::I32Const(0), I::Return, I::End])?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(4, 1, 1, b)
}
fn known_bound(b: &mut Body<'_>, possible: u32) -> Result<(), Diagnostic> {
    b.ops([I::LocalGet(possible), I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(b, 3, 2)?;
    b.op(&I::End)
}
fn complete(b: &mut Body<'_>, output: u32, status: u32, debt: u32) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(status),
        I::If(BlockType::Empty),
        I::GlobalGet(12),
        I::LocalGet(debt),
        I::I32Sub,
        I::GlobalSet(12),
    ])?;
    zero(b, output)?;
    b.ops([
        I::LocalGet(status),
        I::I32Const(1),
        I::I32Eq,
        I::LocalGet(status),
        I::I32Const(2),
        I::I32Eq,
        I::I32Or,
        I::LocalGet(status),
        I::I32Const(255),
        I::I32Eq,
        I::I32Or,
        I::I32Eqz,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::LocalGet(status), I::Return, I::End, I::I32Const(0), I::Return])
}
