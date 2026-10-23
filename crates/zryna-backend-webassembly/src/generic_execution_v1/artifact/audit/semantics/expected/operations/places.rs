//! Independent source operation address/capability obligations.
use super::{
    Diagnostic, Event, Model, VerifiedOperation, copy, drop, frame, invariant, place, raw, value,
};
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let result = op.result().map(|v| value(m, f, v.id())).transpose()?;
    match op.kind() {
        raw::Operation::ReadPlace { place: id, .. } => {
            let src = place(m, f, *id)?;
            copy(
                out,
                frame(src.offset),
                frame(result.ok_or_else(invariant)?.offset),
                src.bytes,
                m,
            )?;
        }
        raw::Operation::InitializePlace { place: id, value: idv } => {
            let src = value(m, f, *idv)?;
            copy(out, frame(src.offset), frame(place(m, f, *id)?.offset), src.bytes, m)?;
        }
        raw::Operation::ReplacePlace { place: id, value: idv } => {
            let dst = place(m, f, *id)?;
            drop(out, dst.offset, dst.ty, m)?;
            let src = value(m, f, *idv)?;
            copy(out, frame(src.offset), frame(dst.offset), src.bytes, m)?;
        }
        raw::Operation::DropPlace { place: id } => {
            let dst = place(m, f, *id)?;
            drop(out, dst.offset, dst.ty, m)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
