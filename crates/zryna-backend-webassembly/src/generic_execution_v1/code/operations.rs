//! First checkpoint instructions retain operation/result/failure binding from the new seal.
use super::{BYTE, Body, Diagnostic, I, Model, WORD, model, source as s};
use crate::generic_execution_v1::invariant;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};
mod calls;
mod construct;
mod containers;
mod loans;
mod partial;
mod push;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| s::value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::Unit => {}
        raw::Operation::I32Literal(n) => literal(b, result.ok_or_else(invariant)?, *n)?,
        raw::Operation::BoolLiteral(v) => literal(b, result.ok_or_else(invariant)?, i32::from(*v))?,
        raw::Operation::ScalarBinary { kind, left, right } => {
            let dst = result.ok_or_else(invariant)?;
            b.addr(0, dst.offset)?;
            s::scalar(b, s::value(m, f, *left)?)?;
            s::scalar(b, s::value(m, f, *right)?)?;
            b.op(&binary(*kind))?;
            b.op(&if dst.bytes == 1 { I::I32Store8(BYTE) } else { I::I32Store(WORD) })?;
        }
        raw::Operation::I32Neg { value } => {
            let dst = result.ok_or_else(invariant)?;
            b.addr(0, dst.offset)?;
            b.op(&I::I32Const(0))?;
            s::scalar(b, s::value(m, f, *value)?)?;
            b.ops([I::I32Sub, I::I32Store(WORD)])?;
        }
        raw::Operation::ReadPlace { place, .. } => {
            s::copy(b, s::place(m, f, *place)?, result.ok_or_else(invariant)?)?;
        }
        raw::Operation::InitializePlace { place, value } => {
            s::copy(b, s::value(m, f, *value)?, s::place(m, f, *place)?)?;
        }
        raw::Operation::ReplacePlace { place, value } => {
            let dst = s::place(m, f, *place)?;
            partial::old_drop(m, b, op)?;
            s::copy(b, s::value(m, f, *value)?, dst)?;
        }
        raw::Operation::DropPlace { .. } => partial::old_drop(m, b, op)?,
        raw::Operation::Clone { source, .. } => {
            let clone = op.clone_frontier().ok_or_else(invariant)?;
            s::referent(m, f, *source)?.address(b, 0)?;
            b.addr(0, result.ok_or_else(invariant)?.offset)?;
            b.op(&I::Call(m.clone_index(clone.ty())))?;
            s::failure(m, b, clone.outer_failure())?;
        }
        raw::Operation::BeginLoan { .. }
        | raw::Operation::EndLoan { .. }
        | raw::Operation::ReadLoan { .. }
        | raw::Operation::ReplaceLoan { .. }
        | raw::Operation::StringConcat { .. } => loans::emit(m, b, op)?,
        raw::Operation::StringFromUtf8 { .. } => {
            m.take(m.memory.claims().literals.len())?;
            let literal = m
                .memory
                .claims()
                .literals
                .iter()
                .find(|l| l.site == op.site())
                .ok_or_else(invariant)?;
            b.ops([
                I::I32Const(
                    crate::generic_execution_v1::checked_u32(literal.offset)?.cast_signed(),
                ),
                I::I32Const(
                    crate::generic_execution_v1::checked_u32(literal.bytes.len())?.cast_signed(),
                ),
            ])?;
            b.addr(0, result.ok_or_else(invariant)?.offset)?;
            b.op(&I::Call(m.string_call(model::STRING_FROM)))?;
            s::failure(m, b, op.failure())?;
        }
        raw::Operation::SourceCall { arguments, .. }
        | raw::Operation::ClosedGenericCall { arguments, .. } => calls::emit(m, b, op, arguments)?,
        raw::Operation::StructConstruct { .. }
        | raw::Operation::ClosedEnumConstruct { .. }
        | raw::Operation::FixedArrayConstruct { .. } => construct::emit(m, b, op)?,
        raw::Operation::VecConstruct { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. } => containers::emit(m, b, op)?,
        raw::Operation::VecPush { .. } => push::emit(m, b, op)?,
        raw::Operation::IndexCopy { base, index, .. } => construct::index(m, b, op, *base, *index)?,
    }
    Ok(())
}
fn literal(
    b: &mut Body<'_>,
    dst: &crate::generic_execution_v1::memory::raw::Slot,
    value: i32,
) -> Result<(), Diagnostic> {
    b.addr(0, dst.offset)?;
    b.ops([I::I32Const(value), if dst.bytes == 1 { I::I32Store8(BYTE) } else { I::I32Store(WORD) }])
}
fn binary(kind: raw::ScalarBinaryKind) -> I<'static> {
    use raw::ScalarBinaryKind as K;
    match kind {
        K::Add => I::I32Add,
        K::Sub => I::I32Sub,
        K::Mul => I::I32Mul,
        K::Eq => I::I32Eq,
        K::Ne => I::I32Ne,
        K::LtS => I::I32LtS,
        K::LeS => I::I32LeS,
        K::GtS => I::I32GtS,
        K::GeS => I::I32GeS,
    }
}
