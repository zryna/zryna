//! Exact operation-branded pending values are committed only after successful acquisition.
use super::{Body, Diagnostic, I, Model, VerifiedOperation, WORD, invariant, s};
use crate::generic_execution_v1::model::containers as C;
use wasm_encoder::MemArg;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let dst = s::value(m, f, op.result().ok_or_else(invariant)?.id())?;
    if let Some(v) = op.vec_construct() {
        let meta = m.element(v.ty())?;
        if meta.element().index() != v.element_type() {
            return Err(invariant());
        }
        b.ops([number(u64::from(meta.element().index()))?, number(v.elements().len() as u64)?])?;
        b.addr(0, dst.offset)?;
        b.op(&I::Call(m.container(C::VEC_ALLOCATE)))?;
        s::failure(m, b, Some(v.outer_failure()))?;
        b.load(0, dst.offset)?;
        b.op(&I::LocalSet(4))?;
        m.take(v.elements().len())?;
        for (index, value) in v.elements().enumerate() {
            let source = s::value(m, f, value.id())?;
            b.addr(0, source.offset)?;
            b.addr(4, crate::generic_execution_v1::checked_u32(index as u64 * meta.stride())?)?;
            b.ops([
                number(u64::from(source.bytes))?,
                I::Call(crate::generic_execution_v1::model::COPY),
            ])?;
        }
        b.addr(0, dst.offset)?;
        b.ops([number(v.elements().len() as u64)?, I::I32Store(MemArg { offset: 4, ..WORD })])?;
    } else if let Some(v) = op.shared_construct() {
        let meta = m.control(v.ty())?;
        if meta.payload().index() != v.referent_type() {
            return Err(invariant());
        }
        b.op(&number(u64::from(m.memory.claims().scratch_start))?)?;
        b.ops([
            number(meta.size())?,
            number(meta.alignment())?,
            I::Call(m.container(C::ALLOCATE_RAW)),
            I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
        ])?;
        b.op(&number(u64::from(m.memory.claims().scratch_start + 4))?)?;
        b.op(&I::I32Load(WORD))?;
        s::failure(m, b, Some(v.outer_failure()))?;
        b.op(&number(u64::from(m.memory.claims().scratch_start))?)?;
        b.ops([I::I32Load(WORD), I::LocalSet(4)])?;
        for offset in [0, 4] {
            b.ops([I::LocalGet(4), I::I32Const(1), I::I32Store(MemArg { offset, ..WORD })])?;
        }
        let source = s::value(m, f, v.payload().id())?;
        b.addr(0, source.offset)?;
        b.addr(4, crate::generic_execution_v1::checked_u32(meta.payload_offset())?)?;
        b.ops([
            number(u64::from(source.bytes))?,
            I::Call(crate::generic_execution_v1::model::COPY),
        ])?;
        b.addr(0, dst.offset)?;
        b.ops([I::LocalGet(4), I::I32Store(WORD)])?;
    } else if let Some(v) = op.weak_downgrade() {
        let source = v.source_place().ok_or_else(invariant)?;
        let source = s::place(m, f, source.id())?;
        b.load(0, source.offset)?;
        b.op(&I::Call(m.container(C::DOWNGRADE)))?;
        s::failure(m, b, Some(v.outer_failure()))?;
        b.addr(0, dst.offset)?;
        b.load(0, source.offset)?;
        b.op(&I::I32Store(WORD))?;
    } else {
        return Err(invariant());
    }
    Ok(())
}
fn number(n: u64) -> Result<I<'static>, Diagnostic> {
    Ok(I::I32Const(crate::generic_execution_v1::checked_u32(n)?.cast_signed()))
}
