//! Retained prepared RHS remains owned until reserve succeeds and append commits.
use super::{Body, Diagnostic, I, Model, WORD, invariant, model, s};
use wasm_encoder::MemArg;
use zryna_ir::generic_v1::execution_v1::VerifiedOperation;
use zryna_ir::generic_v1::execution_v1::raw;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let push = op.vec_push().ok_or_else(invariant)?;
    let source = if let Some(place) = push.target_place() {
        raw::PlaceOrLoan::Place(place.id())
    } else {
        raw::PlaceOrLoan::Loan(push.target_loan().ok_or_else(invariant)?.id())
    };
    let source = s::referent(m, op.site().function, source)?;
    let pending = m.pending(op.site())?;
    let element = m.element(push.ty())?;
    let value = s::value(m, op.site().function, push.value().id())?;
    // Universal element capacity makes length+1 nonwrapping before any mutation.
    source.load(b, 4)?;
    b.ops([
        I::I32Const(
            crate::generic_execution_v1::checked_u32(
                zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS,
            )?
            .cast_signed(),
        ),
        I::I32GeU,
        I::If(wasm_encoder::BlockType::Empty),
    ])?;
    s::cleanup(m, b, push.outer_failure())?;
    b.ops([I::I32Const(3), I::GlobalSet(1), I::I32Const(3), I::Return, I::End])?;
    for offset in [0, 4, 8] {
        b.addr(0, pending.offset + offset)?;
        b.ops([I::I32Const(0), I::I32Store(WORD)])?;
    }
    b.op(&I::I32Const(
        crate::generic_execution_v1::checked_u32(element.element().index())?.cast_signed(),
    ))?;
    source.address(b, 0)?;
    source.load(b, 4)?;
    b.ops([I::I32Const(1), I::I32Add])?;
    b.addr(0, pending.offset)?;
    b.op(&I::Call(m.container(model::containers::VEC_RESERVE)))?;
    s::failure(m, b, Some(push.outer_failure()))?;
    b.load(0, pending.offset)?;
    source.load(b, 4)?;
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(element.stride())?.cast_signed()),
        I::I32Mul,
        I::I32Add,
        I::LocalSet(4),
    ])?;
    b.addr(0, value.offset)?;
    b.ops([
        I::LocalGet(4),
        I::I32Const(crate::generic_execution_v1::checked_u32(value.bytes)?.cast_signed()),
        I::Call(model::COPY),
    ])?;
    b.addr(0, pending.offset)?;
    source.load(b, 4)?;
    b.ops([I::I32Const(1), I::I32Add, I::I32Store(MemArg { offset: 4, ..WORD })])?;
    b.addr(0, pending.offset)?;
    source.address(b, 0)?;
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(pending.bytes)?.cast_signed()),
        I::Call(model::COPY),
    ])
}
