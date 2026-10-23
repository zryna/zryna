//! Independent source operation address/capability obligations.
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, frame, invariant, raw, read, store, value,
};
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::I32Literal(n) => {
            let dst = result.ok_or_else(invariant)?;
            store(out, frame(dst.offset), Expr::Const(i64::from(*n)), 4, m)?;
        }
        raw::Operation::BoolLiteral(n) => {
            let dst = result.ok_or_else(invariant)?;
            store(out, frame(dst.offset), Expr::Const(i64::from(*n)), 1, m)?;
        }
        raw::Operation::ScalarBinary { left, right, .. } => {
            for id in [*left, *right] {
                let src = value(m, f, id)?;
                read(
                    out,
                    frame(src.offset),
                    crate::generic_execution_v1::checked_u8(src.bytes)?,
                    m,
                )?;
            }
            let dst = result.ok_or_else(invariant)?;
            store(
                out,
                frame(dst.offset),
                Expr::Unknown,
                crate::generic_execution_v1::checked_u8(dst.bytes)?,
                m,
            )?;
        }
        raw::Operation::I32Neg { value: id } => {
            let src = value(m, f, *id)?;
            read(out, frame(src.offset), 4, m)?;
            let dst = result.ok_or_else(invariant)?;
            store(
                out,
                frame(dst.offset),
                Expr::Unknown,
                crate::generic_execution_v1::checked_u8(dst.bytes)?,
                m,
            )?;
        }
        raw::Operation::Unit => {}
        _ => return Err(invariant()),
    }
    Ok(())
}
