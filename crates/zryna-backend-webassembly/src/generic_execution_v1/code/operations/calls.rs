//! Private calls use checked source-callee views and statically disjoint invocation frames.
use super::{Body, Diagnostic, I, Model, VerifiedOperation, invariant, model, raw, s};
use zryna_ir::generic_v1::execution_v1::VerifiedParameter;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
    args: &[raw::CallArgument],
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let target = op.call_target().ok_or_else(invariant)?;
    let index = target.index();
    b.addr(0, m.memory.claims().functions[f].bytes)?;
    b.op(&I::LocalSet(4))?;
    m.take(args.len())?;
    for (argument, parameter) in args.iter().zip(target.parameters()) {
        match (argument, parameter) {
            (raw::CallArgument::Value(id), VerifiedParameter::Value(parameter)) => {
                let src = s::value(m, f, *id)?;
                let dst = s::value(m, index, parameter.id())?;
                if src.ty != dst.ty || src.bytes != dst.bytes {
                    return Err(invariant());
                }
                b.addr(0, src.offset)?;
                b.addr(4, dst.offset)?;
                b.ops([
                    I::I32Const(crate::generic_execution_v1::checked_u32(src.bytes)?.cast_signed()),
                    I::Call(model::COPY),
                ])?;
            }
            (raw::CallArgument::Loan(id), VerifiedParameter::Loan(parameter)) => {
                b.addr(4, m.loan_offset(index, parameter.id())?)?;
                b.load(0, m.loan_offset(f, *id)?)?;
                b.op(&I::I32Store(super::super::WORD))?;
            }
            _ => return Err(invariant()),
        }
    }
    b.op(&I::LocalGet(4))?;
    if let Some(result) = op.result() {
        b.addr(0, s::value(m, f, result.id())?.offset)?;
    } else {
        b.op(&I::I32Const(
            crate::generic_execution_v1::checked_u32(m.memory.claims().scratch_start)?
                .cast_signed(),
        ))?;
    }
    b.op(&I::Call(m.source_base + crate::generic_execution_v1::checked_u32(index)?))?;
    s::failure(m, b, op.failure())
}
