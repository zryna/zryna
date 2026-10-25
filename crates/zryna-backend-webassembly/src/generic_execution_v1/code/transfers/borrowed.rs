//! A borrowed arm publishes only its exact selected payload address; no owned payload copy.
use super::{Body, Diagnostic, I, Model, VerifiedBlock, edge, invariant, raw, s};
use wasm_encoder::BlockType;
pub(super) fn emit(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    block: VerifiedBlock<'_, '_>,
) -> Result<(), Diagnostic> {
    let raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } = block.terminator() else {
        return Err(invariant());
    };
    let raw::PlaceOrLoan::Loan(parent) = *scrutinee else { return Err(invariant()) };
    let access = match mode {
        raw::MatchMode::SharedLoan => raw::Access::Shared,
        raw::MatchMode::ExclusiveLoan => raw::Access::Exclusive,
        raw::MatchMode::Value => return Err(invariant()),
    };
    let loan = m.loan(f, parent)?;
    if loan.referent() != *ty || loan.access() != access {
        return Err(invariant());
    }
    let source = s::referent(m, f, *scrutinee)?;
    m.take(arms.len())?;
    for arm in arms {
        source.load(b, 0)?;
        b.ops([
            I::I32Const(crate::generic_execution_v1::checked_u32(arm.ordinal)?.cast_signed()),
            I::I32Eq,
            I::If(BlockType::Empty),
        ])?;
        if let Some(binding) = &arm.binding {
            let raw::MatchBinding::Loan(binding) = binding else { return Err(invariant()) };
            // This origin is independently checked against this exact terminator arm by IR;
            // begin derives the same source parent and static payload offset in target layout.
            s::begin(m, b, f, binding.id)?;
        }
        // Generated Loan bindings have no ordinary SSA parameter and consume no Value lane.
        edge(m, b, f, &arm.edge, 0, 2)?;
        b.op(&I::End)?;
    }
    b.op(&I::Unreachable)
}
