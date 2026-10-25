//! Source-authorized Clone recipes and active-payload reverse cleanup in caller storage.
use super::{Body, Diagnostic, FunctionCode, I, Model, WORD, model};
use crate::generic_execution_v1::invariant;
use wasm_encoder::{BlockType, MemArg};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
mod array;
mod containers;
mod fields;
pub(super) fn clone_helper(m: &Model<'_, '_, '_>, ty: u32) -> Result<FunctionCode, Diagnostic> {
    if m.recursive_count() != 0 {
        return crate::generic_execution_v1::recursive::clone_wrapper(m, ty);
    }
    let mut b = Body::new(m, 2)?;
    let Some(node) = m.clones[ty as usize] else {
        b.op(&I::Unreachable)?;
        return FunctionCode::new(2, 1, 2, b);
    };
    match node.kind() {
        CloneNodeKind::Copy => {
            b.ops([
                I::LocalGet(0),
                I::LocalGet(1),
                I::I32Const(
                    crate::generic_execution_v1::checked_u32(m.types[ty as usize].size())?
                        .cast_signed(),
                ),
                I::Call(model::COPY),
            ])?;
        }
        CloneNodeKind::String => {
            for offset in [0u32, 4, 8] {
                b.load(0, offset)?;
            }
            b.ops([I::LocalGet(1), I::Call(m.string_call(model::STRING_CLONE)), I::Return])?;
        }
        CloneNodeKind::Enum => {
            let offset = crate::generic_execution_v1::checked_u32(
                m.types[ty as usize].payload().ok_or_else(invariant)?.0,
            )?;
            m.take(node.variants().len())?;
            for (ordinal, child) in node.variants() {
                b.ops([
                    I::LocalGet(0),
                    I::I32Load(WORD),
                    I::I32Const(crate::generic_execution_v1::checked_u32(ordinal)?.cast_signed()),
                    I::I32Eq,
                    I::If(BlockType::Empty),
                ])?;
                if let Some(child) = child {
                    child_clone(m, &mut b, child.ty(), offset)?;
                    return_if_failed(&mut b)?;
                }
                b.ops([
                    I::LocalGet(1),
                    I::I32Const(crate::generic_execution_v1::checked_u32(ordinal)?.cast_signed()),
                    I::I32Store(WORD),
                    I::I32Const(0),
                    I::Return,
                    I::End,
                ])?;
            }
            b.op(&I::Unreachable)?;
        }
        CloneNodeKind::Struct => {
            m.take(node.fields().len())?;
            for (ordinal, child) in node.fields() {
                m.take(ordinal as usize + 1)?;
                let offset = crate::generic_execution_v1::checked_u32(
                    m.types[ty as usize].fields().nth(ordinal as usize).ok_or_else(invariant)?.2,
                )?;
                child_clone(m, &mut b, child.ty(), offset)?;
                b.ops([I::LocalTee(2), I::If(BlockType::Empty)])?;
                for (_, prev, prev_offset) in
                    fields::collect(m, ty, ordinal as usize)?.into_iter().rev()
                {
                    b.addr(1, crate::generic_execution_v1::checked_u32(prev_offset)?)?;
                    b.op(&I::Call(m.drop_index(prev.index())))?;
                }
                b.ops([I::LocalGet(2), I::Return, I::End])?;
            }
        }
        CloneNodeKind::FixedArray { length } => array::clone_elements(
            m,
            &mut b,
            ty,
            node.element().ok_or_else(invariant)?.ty(),
            length,
        )?,
        CloneNodeKind::VecEach | CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => {
            containers::clone(m, &mut b, ty, node.kind())?;
        }
    }
    b.op(&I::I32Const(0))?;
    FunctionCode::new(2, 1, 2, b)
}
fn child_clone(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    child: u32,
    offset: u32,
) -> Result<(), Diagnostic> {
    b.addr(0, offset)?;
    b.addr(1, offset)?;
    b.op(&I::Call(m.clone_index(child)))
}
pub(super) fn return_if_failed(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([I::LocalTee(2), I::If(BlockType::Empty), I::LocalGet(2), I::Return, I::End])
}
pub(super) fn drop_helper(m: &Model<'_, '_, '_>, ty: u32) -> Result<FunctionCode, Diagnostic> {
    if m.recursive_count() != 0 {
        return crate::generic_execution_v1::recursive::drop_wrapper(m, ty);
    }
    let mut b = Body::new(m, 1)?;
    let t = m.types[ty as usize];
    match t.category() {
        TypeCategory::Bool | TypeCategory::I32 => {}
        TypeCategory::String => {
            for offset in [0u32, 4, 8] {
                b.load(0, offset)?;
            }
            b.ops([
                I::Call(m.string_call(model::STRING_RELEASE)),
                I::If(BlockType::Empty),
                I::Unreachable,
                I::End,
            ])?;
            for offset in [0u32, 4, 8] {
                b.ops([
                    I::LocalGet(0),
                    I::I32Const(0),
                    I::I32Store(MemArg { offset: u64::from(offset), ..WORD }),
                ])?;
            }
        }
        TypeCategory::Struct => {
            for (_, child, offset) in fields::collect(m, ty, t.fields().len())?.into_iter().rev() {
                b.addr(0, crate::generic_execution_v1::checked_u32(offset)?)?;
                b.op(&I::Call(m.drop_index(child.index())))?;
            }
        }
        TypeCategory::Enum => {
            let offset =
                crate::generic_execution_v1::checked_u32(t.payload().ok_or_else(invariant)?.0)?;
            m.take(t.variants().len())?;
            for (ordinal, child) in t.variants() {
                b.ops([
                    I::LocalGet(0),
                    I::I32Load(WORD),
                    I::I32Const(crate::generic_execution_v1::checked_u32(ordinal)?.cast_signed()),
                    I::I32Eq,
                    I::If(BlockType::Empty),
                ])?;
                if let Some(child) = child {
                    b.addr(0, offset)?;
                    b.op(&I::Call(m.drop_index(child.index())))?;
                }
                b.ops([I::Return, I::End])?;
            }
            b.op(&I::Unreachable)?;
        }
        TypeCategory::FixedArray => array::drop_elements(m, &mut b, ty)?,
        TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
            containers::drop(m, &mut b, ty)?;
        }
    }
    FunctionCode::new(1, 0, 1, b)
}
