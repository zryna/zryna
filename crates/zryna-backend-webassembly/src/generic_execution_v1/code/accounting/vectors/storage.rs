//! Pre-call old storage invariant, exact retained stride/alignment and physical live header.
use super::super::super::WORD;
use super::{BlockType, Body, Diagnostic, I, Model, canonical_failure, number};
use wasm_encoder::MemArg;
use zryna_ownership_runtime_abi::generic_v1::ElementLayout;
pub(super) fn valid(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    row: &ElementLayout,
    pointer: u32,
    length: u32,
    capacity: u32,
) -> Result<(), Diagnostic> {
    let ceiling = zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
        .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride());
    b.ops([
        I::LocalGet(length),
        I::LocalGet(capacity),
        I::I32GtU,
        I::LocalGet(capacity),
        number(ceiling)?,
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(capacity),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(pointer),
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([
        I::End,
        I::Else,
        I::LocalGet(pointer),
        number(u64::from(m.memory.claims().heap_start) + 16)?,
        I::I32LtU,
        I::LocalGet(pointer),
        I::GlobalGet(0),
        I::I32GeU,
        I::I32Or,
        I::LocalGet(pointer),
        I::I32Const(7),
        I::I32And,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    for (offset, value) in [
        (8, 1),
        (12, i32::try_from(row.alignment()).map_err(|_| crate::generic_execution_v1::invariant())?),
    ] {
        b.ops([
            I::LocalGet(pointer),
            I::I32Const(16),
            I::I32Sub,
            I::I32Load(MemArg { offset, ..WORD }),
            I::I32Const(value),
            I::I32Ne,
            I::If(BlockType::Empty),
        ])?;
        canonical_failure(b, 255, 255)?;
        b.op(&I::End)?;
    }
    b.ops([
        I::LocalGet(pointer),
        I::I32Const(16),
        I::I32Sub,
        I::I32Load(MemArg { offset: 4, ..WORD }),
        I::LocalGet(capacity),
        number(row.stride())?,
        I::I32Mul,
        I::I32Ne,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::End])
}
