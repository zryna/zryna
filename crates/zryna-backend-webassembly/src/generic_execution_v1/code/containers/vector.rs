//! Exact runtime element IDs and checked byte products; all outputs start canonical zero.
use super::{
    BlockType, Body, C, Diagnostic, FunctionCode, I, MemArg, Model, WORD, canonical_failure,
    number, scratch, zero,
};
use zryna_layout::StorageTarget;
pub(super) fn allocate(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    zero(&mut b, 2)?;
    b.ops([I::LocalGet(1), I::I32Eqz, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End])?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|v| v.target() == StorageTarget::Linear32V1) {
        b.ops([
            I::LocalGet(0),
            number(u64::from(row.element().index()))?,
            I::I32Eq,
            I::If(BlockType::Empty),
        ])?;
        capacity(&mut b, row.stride(), 1)?;
        scratch(m, &mut b, 0)?;
        b.ops([
            I::LocalGet(1),
            number(row.stride())?,
            I::I32Mul,
            number(row.alignment())?,
            I::Call(m.container(C::ALLOCATE_RAW)),
            I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
        ])?;
        scratch(m, &mut b, 4)?;
        b.ops([I::I32Load(WORD), I::If(BlockType::Empty)])?;
        scratch(m, &mut b, 4)?;
        b.ops([I::I32Load(WORD), I::Return, I::End, I::LocalGet(2)])?;
        scratch(m, &mut b, 0)?;
        b.ops([
            I::I32Load(WORD),
            I::I32Store(WORD),
            I::LocalGet(2),
            I::LocalGet(1),
            I::I32Store(MemArg { offset: 8, ..WORD }),
            I::I32Const(0),
            I::Return,
            I::End,
        ])?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(3, 1, 0, b)
}
fn capacity(b: &mut Body<'_>, stride: u64, local: u32) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(local),
        number((zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16) / stride)?,
        I::I32GtU,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 3, 2)?;
    b.op(&I::End)
}
pub(super) fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    b.ops([I::LocalGet(2), I::LocalGet(3), I::I32GtU, I::If(BlockType::Empty)])?;
    canonical_failure(&mut b, 255, 255)?;
    b.op(&I::End)?;
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|v| v.target() == StorageTarget::Linear32V1) {
        b.ops([
            I::LocalGet(0),
            number(u64::from(row.element().index()))?,
            I::I32Eq,
            I::If(BlockType::Empty),
        ])?;
        capacity(&mut b, row.stride(), 3)?;
        b.ops([
            I::LocalGet(1),
            I::LocalGet(3),
            number(row.stride())?,
            I::I32Mul,
            number(row.alignment())?,
            I::Call(m.container(C::RELEASE_RAW)),
            I::Return,
            I::End,
        ])?;
    }
    canonical_failure(&mut b, 255, 255)?;
    FunctionCode::new(4, 1, 0, b)
}
