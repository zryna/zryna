//! Ordinary edge bytes are snapshotted before generated Shared ownership is published.
use super::{Body, Diagnostic, I, Model, WORD, invariant, model, s};
use wasm_encoder::BlockType;
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedWeakUpgrade, raw};
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    block: VerifiedBlock<'_, '_>,
) -> Result<(), Diagnostic> {
    let upgrade = block.weak_upgrade().ok_or_else(invariant)?;
    let weak = source(m, f, upgrade)?;
    weak.load(b, 0)?;
    b.ops([
        I::Call(m.container(model::containers::WEAK_UPGRADE)),
        I::LocalSet(3),
        I::LocalGet(3),
        I::I32Eqz,
        I::If(BlockType::Empty),
    ])?;
    successor(m, b, f, block.id(), upgrade, true)?;
    b.op(&I::End)?;
    b.ops([I::LocalGet(3), I::I32Const(5), I::I32Eq, I::If(BlockType::Empty)])?;
    successor(m, b, f, block.id(), upgrade, false)?;
    b.op(&I::End)?;
    b.op(&I::LocalGet(3))?;
    s::failure(m, b, Some(upgrade.outer_failure()))
}
fn successor(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    block: u32,
    upgrade: VerifiedWeakUpgrade<'_, '_>,
    success: bool,
) -> Result<(), Diagnostic> {
    let edge = if success { upgrade.success() } else { upgrade.expired() };
    let snapshots = m.snapshot(f, block, success)?;
    m.take(edge.arguments().len())?;
    for (argument, snapshot) in edge.arguments().zip(&snapshots.arguments) {
        s::copy(b, s::value(m, f, argument.id())?, snapshot)?;
    }
    if success {
        let source = source(m, f, upgrade)?;
        let target = s::value(m, f, upgrade.success_binding().id())?;
        b.addr(0, target.offset)?;
        source.load(b, 0)?;
        b.op(&I::I32Store(WORD))?;
    }
    let target = edge.target();
    for (snapshot, parameter) in
        snapshots.arguments.iter().zip(target.value_parameters().skip(usize::from(success)))
    {
        s::copy(b, snapshot, s::value(m, f, parameter.id())?)?;
    }
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(target.id())?.cast_signed()),
        I::LocalSet(2),
        I::Br(2),
    ])
}

fn source(
    m: &Model<'_, '_, '_>,
    f: usize,
    upgrade: VerifiedWeakUpgrade<'_, '_>,
) -> Result<s::Referent, Diagnostic> {
    let source = if let Some(p) = upgrade.source_place() {
        raw::PlaceOrLoan::Place(p.id())
    } else {
        raw::PlaceOrLoan::Loan(upgrade.source_loan().ok_or_else(invariant)?.id())
    };
    s::referent(m, f, source)
}
