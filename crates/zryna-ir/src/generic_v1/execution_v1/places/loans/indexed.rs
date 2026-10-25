//! Dynamic referents retain exact element type and conservative base ancestry.
use super::{
    Failure, LoanRow, Row, TypeWorld, budget, prior, raw, reject, reserve, resource_failure, within,
};
pub(super) fn metadata(
    f: &raw::Function,
    places: &[Row],
    rows: &[LoanRow],
    types: &[raw::ValueType],
    world: &TypeWorld,
    loan: &raw::Loan,
    work: &mut usize,
) -> Result<(Option<u32>, Option<u32>), Failure> {
    budget::charge(work, 6)?;
    let raw::LoanOrigin::Indexed { base, index } = loan.origin else {
        return Err(Failure::InternalFailure);
    };
    let Some(raw::ValueType::Stored(index_ty)) = types.get(index as usize) else {
        return Err(reject("indexed metadata names a foreign or unit index"));
    };
    if world.kind(*index_ty) != Some(super::super::Kind::I32) {
        return Err(reject("indexed metadata index is not exact i32"));
    }
    let (ty, root, parent) = match base {
        raw::PlaceOrLoan::Place(id) => {
            let row =
                places.get(id as usize).ok_or_else(|| reject("indexed base place is foreign"))?;
            (row.ty, Some(row.root), None)
        }
        raw::PlaceOrLoan::Loan(id) => {
            let row = prior(rows, id)?;
            let original = &f.loans[id as usize];
            if !within(original.region.span, loan.region.span)
                || loan.access == raw::Access::Exclusive
                    && original.access != raw::Access::Exclusive
            {
                return Err(reject("indexed child outlives parent or escalates access"));
            }
            (original.referent, row.root, Some(id))
        }
    };
    if !matches!(
        world.kind(ty),
        Some(super::super::Kind::FixedArray { .. } | super::super::Kind::Vec)
    ) {
        return Err(reject("indexed loan base is not exact array or Vec"));
    }
    if world.members(ty).and_then(|m| m.first()).copied().flatten() != Some(loan.referent) {
        return Err(reject("indexed loan referent differs from exact element identity"));
    }
    Ok((root, parent))
}

// Each transient component contains an actual indexed operation. Projections may
// precede or follow that indexed selection in the exact target chain.
// Previously marked chains stop immediately, so total marking work is linear in L.
pub(super) fn transients(
    f: &raw::Function,
    rows: &[LoanRow],
    work: &mut usize,
) -> Result<(), Failure> {
    budget::charge(
        work,
        rows.len()
            .checked_mul(12)
            .ok_or_else(|| resource_failure("indexed transient ancestry work overflow"))?,
    )?;
    let mut marked = reserve(rows.len())?;
    marked.resize(rows.len(), false);
    for (index, loan) in f.loans.iter().enumerate() {
        if loan.region.role != raw::RegionRole::TransientIndexedAccess
            || !matches!(loan.origin, raw::LoanOrigin::Indexed { .. })
        {
            continue;
        }
        let mut at = Some(index);
        while let Some(id) = at {
            if f.loans[id].region.role != raw::RegionRole::TransientIndexedAccess || marked[id] {
                break;
            }
            marked[id] = true;
            at = rows[id].parent.map(|parent| parent as usize);
        }
    }
    for (id, loan) in f.loans.iter().enumerate() {
        if loan.region.role != raw::RegionRole::TransientIndexedAccess {
            continue;
        }
        if let Some(parent) = rows[id].parent {
            let original = &f.loans[parent as usize];
            if original.region.role == raw::RegionRole::TransientIndexedAccess {
                if loan.region.span != original.region.span || loan.access != original.access {
                    return Err(reject("transient chain changes exact region or access"));
                }
                let inherited = marked[parent as usize];
                marked[id] |= inherited;
            }
        }
    }
    if f.loans.iter().zip(marked).any(|(loan, marked)| {
        loan.region.role == raw::RegionRole::TransientIndexedAccess && !marked
    }) {
        return Err(reject("transient projection has no exact indexed descendant"));
    }
    Ok(())
}
