//! Selected Loan arms activate the existing same-seal data-only address, never an owned payload.
use super::{Context, Function, Writer, charge, internal, raw};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedLoanOrigin;
pub(super) fn source(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    scrutinee: raw::PlaceOrLoan,
    mode: raw::MatchMode,
) -> Result<u32, Diagnostic> {
    charge(&c.work, 1)?;
    let raw::PlaceOrLoan::Loan(id) = scrutinee else { return Err(internal()) };
    let loan = f.loan(id)?;
    if loan.function() != f.view.index() || loan.access() != access(mode)? {
        return Err(internal());
    }
    Ok(id)
}
fn access(mode: raw::MatchMode) -> Result<raw::Access, Diagnostic> {
    match mode {
        raw::MatchMode::SharedLoan => Ok(raw::Access::Shared),
        raw::MatchMode::ExclusiveLoan => Ok(raw::Access::Exclusive),
        raw::MatchMode::Value => Err(internal()),
    }
}
pub(super) fn binding(
    c: &Context<'_, '_>,
    f: &Function<'_, '_>,
    parent: u32,
    mode: raw::MatchMode,
    d: &raw::LoanDefinition,
    ordinal: u32,
    w: &mut Writer,
) -> Result<(), Diagnostic> {
    // Four constant visits: retained row, definition/role, exact origin, publication.
    // No ancestry/path is reconstructed here; prologue already paid the exact lens.
    charge(&c.work, 4)?;
    let loan = f.loan(d.id)?;
    if loan.function() != f.view.index()
        || loan.referent() != d.referent
        || loan.access() != d.access
        || d.access != access(mode)?
        || loan.region().role != raw::RegionRole::SelectedMatchArm
    {
        return Err(internal());
    }
    if !matches!(loan.origin(),Some(VerifiedLoanOrigin::StaticProjection {parent:actual,projection:raw::StaticProjection::EnumPayload(variant)})
        if actual.function()==f.view.index() && actual.id()==parent && variant==ordinal)
    {
        return Err(internal());
    }
    super::storage::loans::activate(c, f, loan, w)?;
    w.text(";\n")
}
