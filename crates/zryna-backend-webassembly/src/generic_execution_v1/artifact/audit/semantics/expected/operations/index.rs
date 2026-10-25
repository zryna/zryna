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
                src.load(out, 4, m)?;
                push(
                    out,
                    if src.indirect {
                        Event::LoanVecBounds(0, i64::from(src.offset), 4)
                    } else {
                        Event::VecBounds(0, i64::from(src.offset) + 4)
                    },
                    m,
                )?;
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
            let address = if vector { src.load(out, 0, m)? } else { src.address(out, 0, m)? };
            let index = Expr::Scaled(5, crate::generic_execution_v1::checked_i64(stride)?);
            copy(out, address.add(index), frame(dst.offset), dst.bytes, m)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
