//! Selected tag, exact payload address store, then ordinary successor: derived without code recipes.
use super::super::store;
use super::{
    Diagnostic, Event, Model, Region, VerifiedBlock, edge, frame, invariant, push, raw, referent,
};
pub(super) fn events(
    f: usize,
    block: VerifiedBlock<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
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
    let source = referent(m, f, *scrutinee)?;
    if source.ty != raw::ValueType::Stored(*ty) || !source.indirect {
        return Err(invariant());
    }
    m.take(arms.len())?;
    for arm in arms {
        source.load(out, 0, m)?;
        push(out, Event::LoanTag(0, i64::from(source.offset), 0, i64::from(arm.ordinal)), m)?;
        if let Some(binding) = &arm.binding {
            let raw::MatchBinding::Loan(binding) = binding else { return Err(invariant()) };
            m.take(arm.ordinal as usize + 2)?;
            let layout = m.types.get(*ty as usize).ok_or_else(invariant)?;
            let (_, payload) = layout.variants().nth(arm.ordinal as usize).ok_or_else(invariant)?;
            if payload.map(zryna_layout::generic_v1::TypeId::index) != Some(binding.referent)
                || binding.access != access
            {
                return Err(invariant());
            }
            let child = m.loan(f, binding.id)?;
            let Some(zryna_ir::generic_v1::execution_v1::VerifiedLoanOrigin::StaticProjection {
                parent: actual_parent,
                projection,
            }) = child.origin()
            else {
                return Err(invariant());
            };
            if actual_parent.id() != parent
                || projection != raw::StaticProjection::EnumPayload(arm.ordinal)
                || child.referent() != binding.referent
                || child.access() != access
            {
                return Err(invariant());
            }
            let delta = crate::generic_execution_v1::checked_u32(
                layout.payload().ok_or_else(invariant)?.0,
            )?;
            let address = source.address(out, delta, m)?;
            store(out, frame(m.loan_offset(f, binding.id)?), address, 4, m)?;
        }
        edge(f, &arm.edge, 0, 2, m, out)?;
        push(out, Event::End(Region::Tag), m)?;
    }
    push(out, Event::Trap, m)
}
