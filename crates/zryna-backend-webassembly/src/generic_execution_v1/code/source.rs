//! Exact new-domain CFG, value transfers and retained exit/failure cleanup.
use super::{BYTE, Body, Diagnostic, FunctionCode, I, Model, WORD, model};
use crate::generic_execution_v1::{invariant, memory::raw::Slot};
use wasm_encoder::BlockType;
use zryna_ir::generic_v1::execution_v1::{VerifiedCleanupAction, VerifiedCleanupPlan, raw};
pub(super) fn value<'m>(
    m: &'m Model<'_, '_, '_>,
    function: usize,
    id: u32,
) -> Result<&'m Slot, Diagnostic> {
    m.memory.claims().functions[function].values.get(id as usize).ok_or_else(invariant)
}
pub(super) fn place<'m>(
    m: &'m Model<'_, '_, '_>,
    function: usize,
    id: u32,
) -> Result<&'m Slot, Diagnostic> {
    m.memory.claims().functions[function].places.get(id as usize).ok_or_else(invariant)
}
pub(super) use loans::{Referent, begin, end, end_view, referent};
mod loans;
pub(super) fn scalar(b: &mut Body<'_>, slot: &Slot) -> Result<(), Diagnostic> {
    b.addr(0, slot.offset)?;
    b.op(&if slot.bytes == 1 { I::I32Load8U(BYTE) } else { I::I32Load(WORD) })
}
pub(super) fn copy(b: &mut Body<'_>, src: &Slot, dst: &Slot) -> Result<(), Diagnostic> {
    if src.bytes != dst.bytes {
        return Err(invariant());
    }
    b.addr(0, src.offset)?;
    b.addr(0, dst.offset)?;
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(src.bytes)?.cast_signed()),
        I::Call(model::COPY),
    ])
}
pub(super) fn drop(m: &Model<'_, '_, '_>, b: &mut Body<'_>, slot: &Slot) -> Result<(), Diagnostic> {
    let raw::ValueType::Stored(ty) = slot.ty else { return Ok(()) };
    b.addr(0, slot.offset)?;
    b.op(&I::Call(m.drop_index(ty)))
}
pub(super) fn cleanup(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    plan: VerifiedCleanupPlan<'_, '_>,
) -> Result<(), Diagnostic> {
    m.take(plan.actions().len())?;
    for action in plan.actions() {
        match action {
            VerifiedCleanupAction::DropPlace(p) => drop(m, b, place(m, p.function(), p.id())?)?,
            VerifiedCleanupAction::EndLoan(loan) => end_view(m, b, loan)?,
            VerifiedCleanupAction::DropInitializedPrefix { .. } => return Err(invariant()),
        }
    }
    Ok(())
}
pub(super) fn failure(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    plan: Option<VerifiedCleanupPlan<'_, '_>>,
) -> Result<(), Diagnostic> {
    b.ops([I::LocalTee(3), I::If(BlockType::Empty)])?;
    if let Some(plan) = plan {
        cleanup(m, b, plan)?;
    } else {
        b.op(&I::Unreachable)?;
    }
    b.ops([I::GlobalGet(1), I::Return, I::End])
}
pub(super) fn function(m: &Model<'_, '_, '_>, index: usize) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 4)?;
    let f = m.functions[index];
    b.op(&I::Loop(BlockType::Empty))?;
    for block in f.blocks() {
        b.ops([
            I::LocalGet(2),
            I::I32Const(crate::generic_execution_v1::checked_u32(block.id())?.cast_signed()),
            I::I32Eq,
            I::If(BlockType::Empty),
        ])?;
        for operation in block.operations() {
            super::operations::emit(m, &mut b, operation)?;
        }
        super::transfers::terminator(m, &mut b, index, block)?;
        b.op(&I::End)?;
    }
    b.ops([I::Unreachable, I::End, I::Unreachable])?;
    FunctionCode::new(2, 1, 4, b)
}
