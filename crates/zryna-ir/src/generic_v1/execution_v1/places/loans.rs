//! Loan metadata is checked separately from establishing any liveness.
mod indexed;
mod selected;
use super::{
    Failure, LoanRow, Row, TypeWorld, budget, projection, raw, reject, reserve, resource_failure,
    within,
};

pub(super) fn check(
    f: &raw::Function,
    places: &[Row],
    types: &[raw::ValueType],
    world: &TypeWorld,
    spent: &mut usize,
) -> Result<Vec<LoanRow>, Failure> {
    budget::charge(
        spent,
        f.loans
            .len()
            .checked_mul(7)
            .and_then(|n| n.checked_add(f.parameters.len()))
            .ok_or_else(|| resource_failure("execution loan index overflow"))?,
    )?;
    let mut rows: Vec<LoanRow> = reserve(f.loans.len())?;
    let mut selected_arms = false;
    let mut transients = false;
    for (index, loan) in f.loans.iter().enumerate() {
        crate::generic_v1::execution_v1::typed::loan_sites::role(loan)?;
        selected_arms |= loan.region.role == raw::RegionRole::SelectedMatchArm;
        transients |= loan.region.role == raw::RegionRole::TransientIndexedAccess;
        if loan.id as usize != index
            || world.kind(loan.referent).is_none()
            || !within(f.original_span, loan.region.span)
        {
            return Err(reject("execution loan identity/type/region is foreign"));
        }
        let (root, parent, formal) = origin(f, places, &rows, types, world, loan, spent)?;
        let anchor = parent.map_or(loan.id, |id| rows[id as usize].anchor);
        let inherited = parent.and_then(|id| rows[id as usize].dynamic);
        let dynamic = if let raw::LoanOrigin::Indexed { base, .. } = loan.origin {
            inherited.or(Some(base))
        } else {
            inherited
        };
        rows.push(LoanRow { root, parent, formal, anchor, local_place: None, dynamic });
    }
    parameters(f)?;
    if selected_arms {
        selected::check(f, spent)?;
    }
    if transients {
        indexed::transients(f, &rows, spent)?;
    }
    Ok(rows)
}
fn origin(
    f: &raw::Function,
    places: &[Row],
    rows: &[LoanRow],
    types: &[raw::ValueType],
    world: &TypeWorld,
    loan: &raw::Loan,
    spent: &mut usize,
) -> Result<(Option<u32>, Option<u32>, bool), Failure> {
    Ok(match loan.origin {
        raw::LoanOrigin::Formal(ordinal) => {
            let expected =
                raw::LoanDefinition { id: loan.id, referent: loan.referent, access: loan.access };
            if f.parameters.get(ordinal as usize) != Some(&raw::Parameter::Loan(expected)) {
                return Err(reject("formal loan differs from exact private signature"));
            }
            (None, None, true)
        }
        raw::LoanOrigin::Place(id) => {
            let place =
                places.get(id as usize).ok_or_else(|| reject("loan source place is foreign"))?;
            if place.ty != loan.referent {
                return Err(reject("loan referent differs from exact place type"));
            }
            (Some(place.root), None, false)
        }
        raw::LoanOrigin::InternalReborrow(parent) => {
            let base = prior(rows, parent)?;
            let original = &f.loans[parent as usize];
            if !within(original.region.span, loan.region.span) {
                return Err(reject("child loan outlives exact parent region"));
            }
            if original.referent != loan.referent
                || (loan.access == raw::Access::Exclusive
                    && original.access != raw::Access::Exclusive)
            {
                return Err(reject("reborrow escalates access or changes referent"));
            }
            (base.root, Some(parent), false)
        }
        raw::LoanOrigin::StaticProjection { parent, projection: p } => {
            let base = prior(rows, parent)?;
            let original = &f.loans[parent as usize];
            if !within(original.region.span, loan.region.span) {
                return Err(reject("child loan outlives exact parent region"));
            }
            if projection(world, original.referent, p, spent)? != loan.referent
                || (loan.access == raw::Access::Exclusive
                    && original.access != raw::Access::Exclusive)
            {
                return Err(reject("loan projection changes type/access"));
            }
            (base.root, Some(parent), false)
        }
        raw::LoanOrigin::Indexed { .. } => {
            let (root, parent) = indexed::metadata(f, places, rows, types, world, loan, spent)?;
            (root, parent, false)
        }
    })
}

fn parameters(f: &raw::Function) -> Result<(), Failure> {
    for (ordinal, parameter) in f.parameters.iter().enumerate() {
        if let raw::Parameter::Loan(d) = parameter {
            let loan = f
                .loans
                .get(d.id as usize)
                .ok_or_else(|| reject("signature loan has no arena row"))?;
            if loan.origin
                != raw::LoanOrigin::Formal(
                    u32::try_from(ordinal).map_err(|_| Failure::InternalFailure)?,
                )
                || loan.referent != d.referent
                || loan.access != d.access
            {
                return Err(reject("signature loan differs from exact formal arena row"));
            }
        }
    }
    Ok(())
}

fn prior(rows: &[LoanRow], id: u32) -> Result<&LoanRow, Failure> {
    rows.get(id as usize).ok_or_else(|| reject("loan parent is forward/cyclic/foreign"))
}

#[cfg(test)]
mod tests;

/// Resolve each rooted referent once from already checked parent/type/region metadata.
pub(super) fn retain_referents(
    f: &raw::Function,
    decomposition: &super::decomposition::Decomposition,
    rows: &mut [LoanRow],
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    // Read the source row and prior/canonical row, then write its retained slot.
    budget::charge(
        &mut work,
        rows.len()
            .checked_mul(3)
            .ok_or_else(|| resource_failure("retained loan referent work overflow"))?,
    )?;
    for (index, loan) in f.loans.iter().enumerate() {
        let local_place = match loan.origin {
            raw::LoanOrigin::Formal(_) | raw::LoanOrigin::Indexed { .. } => None,
            raw::LoanOrigin::Place(id) => Some(decomposition.canonical(id)),
            raw::LoanOrigin::InternalReborrow(parent) => rows[parent as usize].local_place,
            raw::LoanOrigin::StaticProjection { parent, projection } => {
                if let Some(place) = rows[parent as usize].local_place {
                    // Each child lookup walks this one canonical sibling list only.
                    // Pay link/ordinal/compare visits before the bounded lookup.
                    budget::charge(
                        &mut work,
                        decomposition
                            .all(place)
                            .len()
                            .checked_mul(3)
                            .and_then(|n| n.checked_add(2))
                            .ok_or_else(|| {
                                resource_failure("retained loan child lookup overflow")
                            })?,
                    )?;
                    Some(decomposition.child(place, projection).ok_or_else(|| {
                        reject("rooted static loan lacks its exact retained source place")
                    })?)
                } else {
                    None
                }
            }
        };
        if rows[index].root.is_some() && local_place.is_none() && rows[index].dynamic.is_none() {
            return Err(reject("rooted loan has no exact retained source place"));
        }
        rows[index].local_place = local_place;
    }
    *spent = work;
    Ok(())
}

#[cfg(test)]
mod indexed_tests;
