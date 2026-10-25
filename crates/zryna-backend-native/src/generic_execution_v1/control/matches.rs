//! Selected payload Loan addresses already belong to the retained physical function plan.
use super::{Frame, edge_transfer, invariant, records};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlagsData, TrapCode, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedLoanOrigin, raw};
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    source: (u32, raw::PlaceOrLoan, raw::MatchMode),
    arms: &[raw::EnumArm],
    blocks: &[Block],
    block_values: &[Vec<u32>],
) -> Result<(), Diagnostic> {
    let (ty, scrutinee, mode) = source;
    let parent = parent(c, ty, scrutinee, mode)?;
    let source = c.source(b, scrutinee)?;
    let view = c.p.stored_type(ty).ok_or_else(invariant)?;
    let tag = b.ins().load(types::I32, MemFlagsData::new(), source, 0);
    for arm in arms {
        let active = b.create_block();
        let next = b.create_block();
        let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(arm.ordinal));
        b.ins().brif(test, active, &[], next, &[]);
        b.switch_to_block(active);
        let skip = match (parent, &arm.binding) {
            (None, Some(raw::MatchBinding::Value(value))) => {
                let destination = c.value(b, value.id)?;
                let payload = records::add(b, source, view.payload().ok_or_else(invariant)?.0)?;
                records::copy(b, payload, destination, c.size(value.ty)?)?;
                1
            }
            (Some(parent), Some(raw::MatchBinding::Loan(loan))) => {
                binding(c, parent, mode, loan, arm.ordinal)?;
                // Loan uses resolve its preplanned address; no generated owned Value exists.
                0
            }
            (_, None) => 0,
            _ => return Err(invariant()),
        };
        edge_transfer(c, b, &arm.edge, skip, blocks, block_values)?;
        b.switch_to_block(next);
    }
    b.ins().trap(TrapCode::unwrap_user(7));
    Ok(())
}
fn access(mode: raw::MatchMode) -> Result<raw::Access, Diagnostic> {
    match mode {
        raw::MatchMode::SharedLoan => Ok(raw::Access::Shared),
        raw::MatchMode::ExclusiveLoan => Ok(raw::Access::Exclusive),
        raw::MatchMode::Value => Err(invariant()),
    }
}
fn parent(
    c: &Frame<'_, '_, '_>,
    ty: u32,
    source: raw::PlaceOrLoan,
    mode: raw::MatchMode,
) -> Result<Option<u32>, Diagnostic> {
    if mode == raw::MatchMode::Value {
        return if matches!(source, raw::PlaceOrLoan::Place(_)) {
            Ok(None)
        } else {
            Err(invariant())
        };
    }
    let raw::PlaceOrLoan::Loan(id) = source else { return Err(invariant()) };
    let f = c.f.source();
    let loan = f.loans().nth(id as usize).ok_or_else(invariant)?;
    if loan.function() != f.index()
        || loan.referent() != ty
        || loan.access() != access(mode)?
        || c.f.loan_type(id) != Some(ty)
        || c.f.loan(id).is_none()
    {
        return Err(invariant());
    }
    Ok(Some(id))
}
fn binding(
    c: &Frame<'_, '_, '_>,
    parent: u32,
    mode: raw::MatchMode,
    definition: &raw::LoanDefinition,
    ordinal: u32,
) -> Result<(), Diagnostic> {
    let f = c.f.source();
    let loan = f.loans().nth(definition.id as usize).ok_or_else(invariant)?;
    if loan.function() != f.index()
        || loan.referent() != definition.referent
        || loan.access() != definition.access
        || definition.access != access(mode)?
        || loan.region().role != raw::RegionRole::SelectedMatchArm
        || c.f.loan_type(loan.id()) != Some(loan.referent())
        || c.f.loan(loan.id()).is_none()
    {
        return Err(invariant());
    }
    if !matches!(loan.origin(), Some(VerifiedLoanOrigin::StaticProjection {
        parent: actual, projection: raw::StaticProjection::EnumPayload(variant)
    }) if actual.function() == f.index() && actual.id() == parent && variant == ordinal)
    {
        return Err(invariant());
    }
    Ok(())
}
