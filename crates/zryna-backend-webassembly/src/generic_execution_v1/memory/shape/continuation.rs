//! Private pending descriptors and edge snapshots derive only from authenticated retained views.
use super::{
    Diagnostic, TypeView, ValueType, VerifiedFunction, add, align, budget, charge, raw, retained,
    slot,
};
pub(super) fn inventory(
    function: VerifiedFunction<'_, '_>,
    types: &[TypeView<'_>],
    cursor: &mut u32,
    work: &mut usize,
) -> Result<(Vec<raw::PendingVec>, Vec<raw::UpgradeEdge>), Diagnostic> {
    let mut pending = Vec::new();
    let mut edges = Vec::new();
    charge(work, function.blocks().len())?;
    for block in function.blocks() {
        charge(work, block.operations().len())?;
        for operation in block.operations() {
            if let Some(push) = operation.vec_push() {
                let mut descriptor = slot(ValueType::Stored(push.ty()), types)?;
                *cursor = align(*cursor, descriptor.alignment)?;
                descriptor.offset = *cursor;
                *cursor = add(*cursor, descriptor.bytes)?;
                retained::<raw::PendingVec>(work, 1)?;
                pending.try_reserve(1).map_err(|_| budget())?;
                pending.push(raw::PendingVec { site: operation.site(), slot: descriptor });
            }
        }
        if let Some(upgrade) = block.weak_upgrade() {
            for success in [false, true] {
                let edge = if success { upgrade.success() } else { upgrade.expired() };
                charge(work, edge.arguments().len())?;
                retained::<raw::Slot>(work, edge.arguments().len())?;
                let mut arguments = Vec::new();
                arguments.try_reserve_exact(edge.arguments().len()).map_err(|_| budget())?;
                for value in edge.arguments() {
                    let mut shape = slot(value.ty(), types)?;
                    *cursor = align(*cursor, shape.alignment)?;
                    shape.offset = *cursor;
                    *cursor = add(*cursor, shape.bytes)?;
                    arguments.push(shape);
                }
                retained::<raw::UpgradeEdge>(work, 1)?;
                edges.try_reserve(1).map_err(|_| budget())?;
                edges.push(raw::UpgradeEdge { block: block.id(), success, arguments });
            }
        }
    }
    Ok((pending, edges))
}
