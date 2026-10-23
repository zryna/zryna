//! Independent source operation address/capability obligations.
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, VerifiedParameter, call, copy, failure,
    frame, invariant, raw, referent, value,
};
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::Clone { source, .. } => {
            let clone = op.clone_frontier().ok_or_else(invariant)?;
            let src = referent(m, f, *source)?;
            call(
                out,
                5 + clone.ty(),
                &[frame(src.offset), frame(result.ok_or_else(invariant)?.offset)],
                m,
            )?;
            failure(out, clone.outer_failure(), m)?;
        }
        raw::Operation::StringFromUtf8 { .. } => {
            m.take(m.memory.claims().literals.len())?;
            let literal = m
                .memory
                .claims()
                .literals
                .iter()
                .find(|l| l.site == op.site())
                .ok_or_else(invariant)?;
            call(
                out,
                1,
                &[
                    Expr::Const(i64::from(literal.offset)),
                    Expr::Const(crate::generic_execution_v1::checked_i64(literal.bytes.len())?),
                    frame(result.ok_or_else(invariant)?.offset),
                ],
                m,
            )?;
            failure(out, op.failure(), m)?;
        }
        raw::Operation::SourceCall { arguments, .. }
        | raw::Operation::ClosedGenericCall { arguments, .. } => {
            let target = op.call_target().ok_or_else(invariant)?;
            m.take(arguments.len())?;
            for (arg, param) in arguments.iter().zip(target.parameters()) {
                let (raw::CallArgument::Value(arg), VerifiedParameter::Value(param)) = (arg, param)
                else {
                    return Err(invariant());
                };
                let src = value(m, f, *arg)?;
                let dst = value(m, target.index(), param.id())?;
                copy(
                    out,
                    frame(src.offset),
                    frame(m.memory.claims().functions[f].bytes + dst.offset),
                    src.bytes,
                    m,
                )?;
            }
            let dst = if let Some(result) = result {
                frame(result.offset)
            } else {
                Expr::Const(i64::from(m.memory.claims().scratch_start))
            };
            call(
                out,
                m.source_base + crate::generic_execution_v1::checked_u32(target.index())?,
                &[frame(m.memory.claims().functions[f].bytes), dst],
                m,
            )?;
            failure(out, op.failure(), m)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
