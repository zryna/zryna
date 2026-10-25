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
            old_drop(op, *id, m, out)?;
            let src = value(m, f, *idv)?;
            copy(out, frame(src.offset), frame(dst.offset), src.bytes, m)?;
        }
        raw::Operation::DropPlace { place: id } => {
            old_drop(op, *id, m, out)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}

fn old_drop(
    op: VerifiedOperation<'_, '_>,
    selected: u32,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    use zryna_ir::generic_v1::execution_v1::VerifiedDropTarget;
    let f = op.site().function;
    let receipt = op.drop_targets().ok_or_else(invariant)?;
    if receipt.is_whole() {
        let slot = place(m, f, selected)?;
        return drop(out, slot.offset, slot.ty, m);
    }
    m.take(
        receipt.targets().len().checked_mul(4).ok_or_else(crate::generic_execution_v1::budget)?,
    )?;
    for target in receipt.targets() {
        let VerifiedDropTarget::Place(target) = target else {
            return Err(invariant());
        };
        if target.function() != f {
            return Err(invariant());
        }
        let slot = place(m, f, target.id())?;
        if slot.ty != raw::ValueType::Stored(target.ty()) {
            return Err(invariant());
        }
        drop(out, slot.offset, raw::ValueType::Stored(target.ty()), m)?;
    }
    Ok(())
}
