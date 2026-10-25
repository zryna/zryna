//! Access rechecked after preparation; paths use only checked static ancestry.
use super::{Context, Failure, State, budget, raw, reject, resource_failure};
#[derive(Clone, Copy)]
enum Path {
    Place(u32),
    Loan(u32),
}
pub(super) fn prepay(
    c: &Context<'_>,
    state: &State,
    scans: usize,
    work: &mut usize,
) -> Result<(), Failure> {
    let paths = c
        .raw()
        .places
        .len()
        .checked_add(c.raw().loans.len())
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| resource_failure("container access path census overflow"))?;
    let visits = state
        .loan_order
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_mul(paths))
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_mul(scans))
        .ok_or_else(|| resource_failure("container access comparison work overflow"))?;
    // Actual alias loops visit only the current live order. One additional visit
    // covers availability/candidate preparation; the full P+L ancestry/path bound
    // and eight units per visit stay unchanged. Census/candidate copies pay full L.
    budget::charge(work, visits)
}
mod credit;
mod indexed;
mod referents;
pub(super) use credit::{instruction_credit, terminator_credit};
pub(super) use indexed::{invalidate, invalidate_call};
pub(super) use referents::referent_place;
impl State {
    fn available(&self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        self.complete(c, id)
    }
    pub(super) fn observation(
        &self,
        c: &Context<'_>,
        source: raw::PlaceOrLoan,
    ) -> Result<(), Failure> {
        let raw::PlaceOrLoan::Place(id) = source else {
            let raw::PlaceOrLoan::Loan(id) = source else { unreachable!() };
            return self.read_loan(c, id);
        };
        self.available(c, id)?;
        for &live in &self.loan_order {
            if c.raw().loans[live as usize].access == raw::Access::Exclusive
                && overlap(c, Path::Place(id), Path::Loan(live))
            {
                return Err(reject("container observation overlaps exclusive loan"));
            }
        }
        Ok(())
    }
    pub(super) fn mutation(
        &self,
        c: &Context<'_>,
        target: raw::PlaceOrLoan,
    ) -> Result<(), Failure> {
        let path = match target {
            raw::PlaceOrLoan::Place(id) => {
                self.available(c, id)?;
                Path::Place(id)
            }
            raw::PlaceOrLoan::Loan(id) => {
                self.read_loan(c, id)?;
                if c.raw().loans[id as usize].access != raw::Access::Exclusive {
                    return Err(reject("append requires live exact exclusive loan"));
                }
                Path::Loan(id)
            }
        };
        for &live in &self.loan_order {
            if let raw::PlaceOrLoan::Loan(id) = target
                && (live == id || ancestor(c, id, live))
            {
                continue;
            }
            if overlap(c, path, Path::Loan(live)) {
                return Err(reject("append target overlaps a live loan"));
            }
        }
        Ok(())
    }
}
fn ancestor(c: &Context<'_>, id: u32, sought: u32) -> bool {
    let mut at = c.checked().places.loans[id as usize].parent;
    while let Some(parent) = at {
        if parent == sought {
            return true;
        }
        at = c.checked().places.loans[parent as usize].parent;
    }
    false
}
fn root(c: &Context<'_>, p: Path) -> (bool, u32) {
    match p {
        Path::Place(id) => (true, c.checked().places.rows[id as usize].root),
        Path::Loan(id) => {
            let row = &c.checked().places.loans[id as usize];
            row.root.map_or((false, row.anchor), |id| (true, id))
        }
    }
}
fn step(c: &Context<'_>, mut p: Path) -> Option<(Path, raw::StaticProjection)> {
    loop {
        match p {
            Path::Place(id) => {
                return match c.raw().places[id as usize].kind {
                    raw::PlaceKind::StructField { base, ordinal } => {
                        Some((Path::Place(base), raw::StaticProjection::StructField(ordinal)))
                    }
                    raw::PlaceKind::EnumPayload { base, variant } => {
                        Some((Path::Place(base), raw::StaticProjection::EnumPayload(variant)))
                    }
                    raw::PlaceKind::FixedArrayConstant { base, index } => {
                        Some((Path::Place(base), raw::StaticProjection::FixedArrayConstant(index)))
                    }
                    _ => None,
                };
            }
            Path::Loan(id) => {
                if let Some(base) = c.checked().places.loans[id as usize].dynamic {
                    p = match base {
                        raw::PlaceOrLoan::Place(id) => Path::Place(id),
                        raw::PlaceOrLoan::Loan(id) => Path::Loan(id),
                    };
                    continue;
                }
                match c.raw().loans[id as usize].origin {
                    raw::LoanOrigin::Formal(_) => return None,
                    raw::LoanOrigin::Place(id) => p = Path::Place(id),
                    raw::LoanOrigin::InternalReborrow(id) => p = Path::Loan(id),
                    raw::LoanOrigin::StaticProjection { parent, projection } => {
                        return Some((Path::Loan(parent), projection));
                    }
                    raw::LoanOrigin::Indexed { base, .. } => {
                        p = match base {
                            raw::PlaceOrLoan::Place(id) => Path::Place(id),
                            raw::PlaceOrLoan::Loan(id) => Path::Loan(id),
                        }
                    }
                }
            }
        }
    }
}
fn depth(c: &Context<'_>, mut p: Path) -> usize {
    let mut n = 0;
    while let Some((parent, _)) = step(c, p) {
        n += 1;
        p = parent;
    }
    n
}
fn overlap(c: &Context<'_>, mut left: Path, mut right: Path) -> bool {
    if root(c, left) != root(c, right) {
        return false;
    }
    let (mut a, mut b) = (depth(c, left), depth(c, right));
    while a > b {
        left = step(c, left).expect("checked positive depth").0;
        a -= 1;
    }
    while b > a {
        right = step(c, right).expect("checked positive depth").0;
        b -= 1;
    }
    while let (Some((lp, ls)), Some((rp, rs))) = (step(c, left), step(c, right)) {
        if ls != rs {
            return false;
        }
        left = lp;
        right = rp;
    }
    true
}

/// Complete static paths, including duplicate raw IDs, denote the same referent.
pub(super) fn overlaps(c: &Context<'_>, left: raw::PlaceOrLoan, right: raw::PlaceOrLoan) -> bool {
    let path = |source| match source {
        raw::PlaceOrLoan::Place(id) => Path::Place(id),
        raw::PlaceOrLoan::Loan(id) => Path::Loan(id),
    };
    overlap(c, path(left), path(right))
}
pub(super) fn free(c: &Context<'_>, state: &State, id: u32, moving: bool) -> Result<(), Failure> {
    for &loan in &state.loan_order {
        if (moving || c.raw().loans[loan as usize].access == raw::Access::Exclusive)
            && overlaps(c, raw::PlaceOrLoan::Place(id), raw::PlaceOrLoan::Loan(loan))
        {
            return Err(reject("place access conflicts with a live overlapping loan"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod credit_tests;
