//! Independent source operation address/capability obligations.
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, copy, frame, invariant, raw, store, value,
};
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::StructConstruct { ty, fields } => {
            m.take(fields.len())?;
            for ((_, _, offset), v) in m.types[*ty as usize].fields().zip(fields) {
                let src = value(m, f, *v)?;
                copy(
                    out,
                    frame(src.offset),
                    frame(
                        result.ok_or_else(invariant)?.offset
                            + crate::generic_execution_v1::checked_u32(offset)?,
                    ),
                    src.bytes,
                    m,
                )?;
            }
        }
        raw::Operation::ClosedEnumConstruct { ty, payload, ordinal } => {
            if let Some(v) = payload {
                let src = value(m, f, *v)?;
                copy(
                    out,
                    frame(src.offset),
                    frame(
                        result.ok_or_else(invariant)?.offset
                            + crate::generic_execution_v1::checked_u32(
                                m.types[*ty as usize].payload().ok_or_else(invariant)?.0,
                            )?,
                    ),
                    src.bytes,
                    m,
                )?;
            }
            store(
                out,
                frame(result.ok_or_else(invariant)?.offset),
                Expr::Const(i64::from(*ordinal)),
                4,
                m,
            )?;
        }
        raw::Operation::FixedArrayConstruct { ty, elements } => {
            m.take(elements.len())?;
            for (index, v) in elements.iter().enumerate() {
                let src = value(m, f, *v)?;
                let offset =
                    index as u64 * m.types[*ty as usize].array_stride().ok_or_else(invariant)?;
                copy(
                    out,
                    frame(src.offset),
                    frame(
                        result.ok_or_else(invariant)?.offset
                            + crate::generic_execution_v1::checked_u32(offset)?,
                    ),
                    src.bytes,
                    m,
                )?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
