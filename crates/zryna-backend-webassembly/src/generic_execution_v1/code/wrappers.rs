//! Scalar-only ordinary exports with checked ownership reset and canonical post-cleanup trap.
use super::{BYTE, Body, Diagnostic, FunctionCode, I, Model, WORD};
use crate::generic_execution_v1::{budget, invariant};
use wasm_encoder::BlockType;
use zryna_ir::generic_v1::execution_v1::{VerifiedParameter, raw};
pub(super) fn wrapper(m: &Model<'_, '_, '_>, index: usize) -> Result<FunctionCode, Diagnostic> {
    let f = m.functions[index];
    let count = crate::generic_execution_v1::checked_u32(f.parameters().len())?;
    // Runtime packed results and cleanup booleans occupy scratch bytes 0..8.
    // Keep the scalar public result disjoint inside the existing verified 32-byte interval.
    let result_out = m.memory.claims().scratch_start.checked_add(16).ok_or_else(budget)?;
    let mut b = Body::new(m, 1)?;
    if m.accounted() {
        b.ops([I::GlobalGet(12), I::If(BlockType::Empty), I::Unreachable, I::End])?;
    }
    b.ops([
        I::GlobalGet(2),
        I::GlobalGet(8),
        I::I32Or,
        I::If(BlockType::Empty),
        I::Unreachable,
        I::End,
        I::I32Const(1),
        I::GlobalSet(2),
    ])?;
    if m.accounted() {
        for global in [10, 11, 12] {
            b.ops([I::I32Const(0), I::GlobalSet(global)])?;
        }
    }
    for global in [1, 5, 6, 7, 9] {
        b.ops([I::I32Const(0), I::GlobalSet(global)])?;
    }
    b.ops([
        I::I32Const(
            crate::generic_execution_v1::checked_u32(m.memory.claims().heap_start)?.cast_signed(),
        ),
        I::GlobalSet(0),
    ])?;
    for (ordinal, parameter) in f.parameters().enumerate() {
        let VerifiedParameter::Value(value) = parameter else { return Err(invariant()) };
        let slot = &m.memory.claims().functions[index].values[value.id() as usize];
        if slot.bytes == 1 {
            b.ops([
                I::LocalGet(crate::generic_execution_v1::checked_u32(ordinal)?),
                I::I32Const(1),
                I::I32GtU,
                I::If(BlockType::Empty),
                I::I32Const(0),
                I::GlobalSet(2),
                I::Unreachable,
                I::End,
            ])?;
        }
        b.ops([
            I::I32Const(
                crate::generic_execution_v1::checked_u32(
                    m.memory.claims().frames_start + slot.offset,
                )?
                .cast_signed(),
            ),
            I::LocalGet(crate::generic_execution_v1::checked_u32(ordinal)?),
            if slot.bytes == 1 {
                I::I32Store8(BYTE)
            } else if slot.bytes == 4 {
                I::I32Store(WORD)
            } else {
                return Err(invariant());
            },
        ])?;
    }
    b.ops([
        I::I32Const(
            crate::generic_execution_v1::checked_u32(m.memory.claims().frames_start)?.cast_signed(),
        ),
        I::I32Const(crate::generic_execution_v1::checked_u32(result_out)?.cast_signed()),
        I::Call(m.source_base + crate::generic_execution_v1::checked_u32(index)?),
        I::LocalSet(count),
        I::I32Const(0),
        I::GlobalSet(2),
    ])?;
    if m.accounted() {
        b.ops([
            I::GlobalGet(8),
            I::GlobalGet(12),
            I::I32Or,
            I::If(BlockType::Empty),
            I::Unreachable,
            I::End,
        ])?;
    }
    b.ops([I::LocalGet(count), I::If(BlockType::Empty), I::Unreachable, I::End])?;
    b.ops([I::GlobalGet(8), I::If(BlockType::Empty), I::Unreachable, I::End])?;
    let raw::ValueType::Stored(ty) = f.result_type() else { return Err(invariant()) };
    let t = m.types[ty as usize];
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(result_out)?.cast_signed()),
        if t.size() == 1 {
            I::I32Load8U(BYTE)
        } else if t.size() == 4 {
            I::I32Load(WORD)
        } else {
            return Err(invariant());
        },
    ])?;
    FunctionCode::new(count, 1, 1, b)
}
