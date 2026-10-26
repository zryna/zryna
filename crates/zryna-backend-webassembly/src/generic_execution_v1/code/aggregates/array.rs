//! Fixed arrays keep actual preparation progress in invocation-local Wasm locals.
use super::{BlockType, Body, Diagnostic, I, Model, invariant};
pub(super) fn clone_elements(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    ty: u32,
    child: u32,
    length: u64,
) -> Result<(), Diagnostic> {
    let stride = crate::generic_execution_v1::checked_u32(
        m.types[ty as usize].array_stride().ok_or_else(invariant)?,
    )?
    .cast_signed();
    b.ops([
        I::Block(BlockType::Empty),
        I::Loop(BlockType::Empty),
        I::LocalGet(3),
        I::I32Const(crate::generic_execution_v1::checked_u32(length)?.cast_signed()),
        I::I32GeU,
        I::BrIf(1),
    ])?;
    for local in [0, 1] {
        b.ops([I::LocalGet(local), I::LocalGet(3), I::I32Const(stride), I::I32Mul, I::I32Add])?;
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
        I::LocalGet(1),
        I::LocalGet(3),
        I::I32Const(stride),
        I::I32Mul,
        I::I32Add,
        I::Call(m.drop_index(child)),
        I::Br(0),
        I::End,
        I::End,
        I::LocalGet(2),
        I::Return,
        I::End,
        I::LocalGet(3),
        I::I32Const(1),
        I::I32Add,
        I::LocalSet(3),
        I::Br(0),
        I::End,
        I::End,
    ])
}
pub(super) fn drop_elements(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    ty: u32,
) -> Result<(), Diagnostic> {
    let t = m.types[ty as usize];
    let child = t.referenced_type().ok_or_else(invariant)?.index();
    b.ops([
        I::I32Const(
            crate::generic_execution_v1::checked_u32(t.array_length().ok_or_else(invariant)?)?
                .cast_signed(),
        ),
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
        I::I32Const(
            crate::generic_execution_v1::checked_u32(t.array_stride().ok_or_else(invariant)?)?
                .cast_signed(),
        ),
        I::I32Mul,
        I::LocalGet(0),
        I::I32Add,
        I::Call(m.drop_index(child)),
        I::Br(0),
        I::End,
        I::End,
    ])
}
