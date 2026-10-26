//! Constant selected-edge checks against exact retained same-function Loan metadata.
use super::{Diagnostic, VerifiedFunction, invariant, raw};
use zryna_ir::generic_v1::execution_v1::VerifiedLoanOrigin;
fn access(mode: raw::MatchMode) -> Result<raw::Access, Diagnostic> {
    match mode {
        raw::MatchMode::SharedLoan => Ok(raw::Access::Shared),
        raw::MatchMode::ExclusiveLoan => Ok(raw::Access::Exclusive),
        raw::MatchMode::Value => Err(invariant()),
    }
}
pub(super) fn source(
    f: VerifiedFunction<'_, '_>,
    ty: u32,
    source: raw::PlaceOrLoan,
    mode: raw::MatchMode,
) -> Result<u32, Diagnostic> {
    let raw::PlaceOrLoan::Loan(id) = source else { return Err(invariant()) };
    // The existing slice/map loans iterator implements constant-time nth.
    let loan = f.loans().nth(id as usize).ok_or_else(invariant)?;
    if loan.function() != f.index() || loan.referent() != ty || loan.access() != access(mode)? {
        return Err(invariant());
    }
    Ok(id)
}
pub(super) fn binding(
    f: VerifiedFunction<'_, '_>,
    parent: u32,
    mode: raw::MatchMode,
    definition: &raw::LoanDefinition,
    ordinal: u32,
) -> Result<(), Diagnostic> {
    let loan = f.loans().nth(definition.id as usize).ok_or_else(invariant)?;
    if loan.function() != f.index()
        || loan.referent() != definition.referent
        || loan.access() != definition.access
        || definition.access != access(mode)?
        || loan.region().role != raw::RegionRole::SelectedMatchArm
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
