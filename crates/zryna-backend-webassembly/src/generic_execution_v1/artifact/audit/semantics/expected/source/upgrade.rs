//! Exact status edges and snapshot-before-binding obligations for source-authorized upgrade.
use super::super::{call, failure, store};
use super::{Diagnostic, Event, Model, Region, VerifiedBlock, copy, frame, invariant, push, value};
use crate::generic_execution_v1::model::containers as C;
use zryna_ir::generic_v1::execution_v1::VerifiedWeakUpgrade;
pub(super) fn events(
    f: usize,
    block: VerifiedBlock<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let upgrade = block.weak_upgrade().ok_or_else(invariant)?;
    let source = source(m, f, upgrade)?;
    let pointer = source.load(out, 0, m)?;
    call(out, m.container(C::WEAK_UPGRADE), &[pointer], m)?;
    for success in [true, false] {
        push(out, if success { Event::UpgradeSuccess } else { Event::UpgradeExpired }, m)?;
        successor(f, block.id(), upgrade, success, m, out)?;
        push(
            out,
            Event::End(if success { Region::UpgradeSuccess } else { Region::UpgradeExpired }),
            m,
        )?;
    }
    failure(out, Some(upgrade.outer_failure()), m)
}
fn successor(
    f: usize,
    block: u32,
    upgrade: VerifiedWeakUpgrade<'_, '_>,
    success: bool,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let edge = if success { upgrade.success() } else { upgrade.expired() };
    let snapshots = m.snapshot(f, block, success)?;
    m.take(edge.arguments().len())?;
    for (argument, snapshot) in edge.arguments().zip(&snapshots.arguments) {
        let source = value(m, f, argument.id())?;
        copy(out, frame(source.offset), frame(snapshot.offset), source.bytes, m)?;
    }
    if success {
        let source = source(m, f, upgrade)?;
        let target = value(m, f, upgrade.success_binding().id())?;
        let pointer = source.load(out, 0, m)?;
        store(out, frame(target.offset), pointer, 4, m)?;
    }
    let target = edge.target();
    m.take(snapshots.arguments.len())?;
    for (snapshot, parameter) in
        snapshots.arguments.iter().zip(target.value_parameters().skip(usize::from(success)))
    {
        let target = value(m, f, parameter.id())?;
        copy(out, frame(snapshot.offset), frame(target.offset), snapshot.bytes, m)?;
    }
    push(out, Event::Edge(i64::from(target.id())), m)?;
    push(out, Event::Jump(2), m)
}

fn source(
    m: &Model<'_, '_, '_>,
    f: usize,
    upgrade: VerifiedWeakUpgrade<'_, '_>,
) -> Result<super::super::loans::Referent, Diagnostic> {
    use zryna_ir::generic_v1::execution_v1::raw::PlaceOrLoan;
    let source = if let Some(p) = upgrade.source_place() {
        PlaceOrLoan::Place(p.id())
    } else {
        PlaceOrLoan::Loan(upgrade.source_loan().ok_or_else(invariant)?.id())
    };
    super::super::referent(m, f, source)
}
