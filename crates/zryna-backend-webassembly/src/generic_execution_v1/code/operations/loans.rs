//! Retained nonstored loan roles and old-drop-before-replacement commitment.
use super::{Body, Diagnostic, I, Model, VerifiedOperation, WORD, invariant, model, raw, s};
mod indexed;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    match *op.kind() {
        raw::Operation::BeginLoan { loan, failure: None } => s::begin(m, b, f, loan)?,
        raw::Operation::BeginLoan { failure: Some(_), .. } => indexed::begin(m, b, op)?,
        raw::Operation::EndLoan { loan } => s::end(m, b, loan, f)?,
        raw::Operation::ReadLoan { loan } => {
            let source = s::referent(m, f, raw::PlaceOrLoan::Loan(loan))?;
            source.address(b, 0)?;
            b.addr(0, s::value(m, f, op.result().ok_or_else(invariant)?.id())?.offset)?;
            b.ops([
                I::I32Const(crate::generic_execution_v1::checked_u32(source.bytes)?.cast_signed()),
                I::Call(model::COPY),
            ])?;
        }
        raw::Operation::ReplaceLoan { loan, value } => {
            let receipt = op.drop_targets().ok_or_else(invariant)?;
            if !receipt.is_whole() {
                return Err(invariant());
            }
            let source = s::referent(m, f, raw::PlaceOrLoan::Loan(loan))?;
            m.take(receipt.targets().len())?;
            for target in receipt.targets() {
                let zryna_ir::generic_v1::execution_v1::VerifiedDropTarget::Loan(target) = target
                else {
                    return Err(invariant());
                };
                if target.id() != loan || target.function() != f {
                    return Err(invariant());
                }
                source.address(b, 0)?;
                b.op(&I::Call(m.drop_index(target.referent())))?;
            }
            let value = s::value(m, f, value)?;
            b.addr(0, value.offset)?;
            source.address(b, 0)?;
            b.ops([
                I::I32Const(crate::generic_execution_v1::checked_u32(value.bytes)?.cast_signed()),
                I::Call(model::COPY),
            ])?;
        }
        raw::Operation::StringConcat { left, right, .. } => {
            let target = s::value(m, f, op.result().ok_or_else(invariant)?.id())?;
            for offset in [0, 4, 8] {
                b.addr(0, target.offset + offset)?;
                b.ops([I::I32Const(0), I::I32Store(WORD)])?;
            }
            s::referent(m, f, left)?.address(b, 0)?;
            s::referent(m, f, right)?.address(b, 0)?;
            b.addr(0, target.offset)?;
            b.op(&I::Call(m.string_call(m.concat_index())))?;
            s::failure(m, b, op.failure())?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
