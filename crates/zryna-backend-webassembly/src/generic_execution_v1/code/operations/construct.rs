//! Inline carriers use exact same-universe offsets/stride; constructors allocate no boxes.
use super::{Body, Diagnostic, I, Model, VerifiedOperation, WORD, invariant, model, raw, s};
use wasm_encoder::BlockType;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let dst = s::value(m, f, op.result().ok_or_else(invariant)?.id())?;
    match op.kind() {
        raw::Operation::StructConstruct { ty, fields } => {
            for ((_, _, offset), value) in m.types[*ty as usize].fields().zip(fields) {
                part(
                    m,
                    b,
                    f,
                    *value,
                    dst.offset + crate::generic_execution_v1::checked_u32(offset)?,
                )?;
            }
        }
        raw::Operation::ClosedEnumConstruct { ty, ordinal, payload } => {
            if let Some(value) = payload {
                part(
                    m,
                    b,
                    f,
                    *value,
                    dst.offset
                        + crate::generic_execution_v1::checked_u32(
                            m.types[*ty as usize].payload().ok_or_else(invariant)?.0,
                        )?,
                )?;
            }
            b.addr(0, dst.offset)?;
            b.ops([
                I::I32Const(crate::generic_execution_v1::checked_u32(*ordinal)?.cast_signed()),
                I::I32Store(WORD),
            ])?;
        }
        raw::Operation::FixedArrayConstruct { ty, elements } => {
            let stride = m.types[*ty as usize].array_stride().ok_or_else(invariant)?;
            for (index, value) in elements.iter().enumerate() {
                part(
                    m,
                    b,
                    f,
                    *value,
                    dst.offset + crate::generic_execution_v1::checked_u32(index as u64 * stride)?,
                )?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn part(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    value: u32,
    offset: u32,
) -> Result<(), Diagnostic> {
    let src = s::value(m, f, value)?;
    b.addr(0, src.offset)?;
    b.addr(0, offset)?;
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(src.bytes)?.cast_signed()),
        I::Call(model::COPY),
    ])
}
pub(super) fn index(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
    base: raw::PlaceOrLoan,
    index: u32,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let base = s::referent(m, f, base)?;
    let raw::ValueType::Stored(ty) = base.ty else { return Err(invariant()) };
    let t = m.types[ty as usize];
    let vector = t.category() == zryna_layout::TypeCategory::Vec;
    let stride =
        if vector { m.element(ty)?.stride() } else { t.array_stride().ok_or_else(invariant)? };
    s::scalar(b, s::value(m, f, index)?)?;
    b.ops([I::LocalTee(5)])?;
    if vector {
        base.load(b, 4)?;
    } else {
        b.op(&I::I32Const(
            crate::generic_execution_v1::checked_u32(t.array_length().ok_or_else(invariant)?)?
                .cast_signed(),
        ))?;
    }
    b.ops([I::I32GeU, I::If(BlockType::Empty)])?;
    s::cleanup(m, b, op.failure().ok_or_else(invariant)?)?;
    b.ops([I::I32Const(1), I::GlobalSet(1), I::I32Const(1), I::Return, I::End])?;
    let dst = s::value(m, f, op.result().ok_or_else(invariant)?.id())?;
    if vector {
        base.load(b, 0)?;
    } else {
        base.address(b, 0)?;
    }
    b.ops([
        I::LocalGet(5),
        I::I32Const(crate::generic_execution_v1::checked_u32(stride)?.cast_signed()),
        I::I32Mul,
        I::I32Add,
    ])?;
    b.addr(0, dst.offset)?;
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(dst.bytes)?.cast_signed()),
        I::Call(model::COPY),
    ])
}
