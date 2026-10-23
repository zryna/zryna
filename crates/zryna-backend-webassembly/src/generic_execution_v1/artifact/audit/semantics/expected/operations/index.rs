//! Independent source operation address/capability obligations.
use super::{
    Diagnostic, Event, Expr, Model, Region, VerifiedOperation, cleanup, copy, frame, invariant,
    push, raw, read, referent, value,
};
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::IndexCopy { base, index, .. } => {
            let src = referent(m, f, *base)?;
            let raw::ValueType::Stored(ty) = src.ty else { return Err(invariant()) };
            let t = m.types[ty as usize];
            read(out, frame(value(m, f, *index)?.offset), 4, m)?;
            push(out, Event::Index(Expr::Load(0, i64::from(value(m, f, *index)?.offset), 4)), m)?;
            let vector = t.category() == zryna_layout::TypeCategory::Vec;
            let stride = if vector {
                m.element(ty)?.stride()
            } else {
                t.array_stride().ok_or_else(invariant)?
            };
            if vector {
                read(out, frame(src.offset + 4), 4, m)?;
                push(out, Event::VecBounds(0, i64::from(src.offset) + 4), m)?;
            } else {
                push(
                    out,
                    Event::Bounds(crate::generic_execution_v1::checked_i64(
                        t.array_length().ok_or_else(invariant)?,
                    )?),
                    m,
                )?;
            }
            cleanup(out, op.failure().ok_or_else(invariant)?, m)?;
            push(out, Event::Status(1), m)?;
            push(out, Event::TrapReturn(1), m)?;
            push(out, Event::End(Region::Bounds), m)?;
            let dst = result.ok_or_else(invariant)?;
            if vector {
                read(out, frame(src.offset), 4, m)?;
            }
            copy(
                out,
                if vector {
                    Expr::IndirectIndexed(
                        0,
                        i64::from(src.offset),
                        5,
                        crate::generic_execution_v1::checked_i64(stride)?,
                    )
                } else {
                    Expr::Indexed(
                        0,
                        i64::from(src.offset),
                        5,
                        crate::generic_execution_v1::checked_i64(stride)?,
                    )
                },
                frame(dst.offset),
                dst.bytes,
                m,
            )?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
