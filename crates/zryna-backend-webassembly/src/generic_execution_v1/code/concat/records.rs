//! Validate each separate canonical String span before any source byte is copied.
use super::{BlockType, Body, Diagnostic, I, MemArg, Model, WORD, canonical_failure};
pub(super) fn valid(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    pointer: u32,
) -> Result<(), Diagnostic> {
    let length = pointer + 1;
    let capacity = pointer + 2;
    b.ops([
        I::LocalGet(pointer),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(length),
        I::LocalGet(capacity),
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::Else, I::LocalGet(length), I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(b, 255, 255)?;
    b.ops([
        I::End,
        I::LocalGet(pointer),
        I::I32Const(
            crate::generic_execution_v1::checked_u32(m.memory.claims().heap_start + 16)?
                .cast_signed(),
        ),
        I::I32LtU,
        I::LocalGet(pointer),
        I::GlobalGet(0),
        I::I32GeU,
        I::I32Or,
        I::LocalGet(pointer),
        I::I32Const(7),
        I::I32And,
        I::I32Or,
        I::LocalGet(pointer),
        I::LocalGet(length),
        I::I32Add,
        I::LocalGet(pointer),
        I::I32LtU,
        I::I32Or,
        I::LocalGet(pointer),
        I::LocalGet(length),
        I::I32Add,
        I::GlobalGet(0),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    for (offset, expected) in [(8, None), (4, Some(length))] {
        b.ops([
            I::LocalGet(pointer),
            I::I32Const(16),
            I::I32Sub,
            I::I32Load(MemArg { offset, ..WORD }),
        ])?;
        b.op(&expected.map_or(I::I32Const(1), I::LocalGet))?;
        b.ops([I::I32Ne, I::If(BlockType::Empty)])?;
        canonical_failure(b, 255, 255)?;
        b.op(&I::End)?;
    }
    b.ops([I::LocalGet(length), I::LocalGet(capacity), I::I32Ne, I::If(BlockType::Empty)])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::End])
}
