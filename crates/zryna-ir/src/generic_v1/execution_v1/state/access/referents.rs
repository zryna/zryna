//! Exact rooted referents are retained by checked metadata; formals stay caller-owned.
use super::{Context, Failure, State, reject};
pub(in crate::generic_v1::execution_v1::state) fn referent_place(
    c: &Context<'_>,
    id: u32,
) -> Result<Option<u32>, Failure> {
    let row = c
        .checked()
        .places
        .loans
        .get(id as usize)
        .ok_or_else(|| reject("loan referent identity is foreign"))?;
    if row.root.is_some() && row.local_place.is_none() && row.dynamic.is_none() {
        return Err(reject("checked rooted loan has no retained source place"));
    }
    Ok(row.local_place)
}
impl State {
    pub(in crate::generic_v1::execution_v1::state) fn loan_complete(
        &self,
        c: &Context<'_>,
        id: u32,
    ) -> Result<(), Failure> {
        self.loan_complete_begin(c, id, false)
    }
    pub(in crate::generic_v1::execution_v1::state) fn loan_complete_begin(
        &self,
        c: &Context<'_>,
        id: u32,
        selected: bool,
    ) -> Result<(), Failure> {
        if let Some(place) = referent_place(c, id)?
            && self.status(c, place).0 != super::super::storage::Init::Whole
        {
            return Err(reject("loan referent is incomplete, uninitialized or moved"));
        }
        let mut at = id;
        loop {
            let loan = &c.raw().loans[at as usize];
            match loan.origin {
                super::raw::LoanOrigin::Formal(_) => return Ok(()),
                super::raw::LoanOrigin::Place(place) => return self.active(c, place),
                super::raw::LoanOrigin::InternalReborrow(parent) => at = parent,
                super::raw::LoanOrigin::StaticProjection { parent, projection } => {
                    if matches!(projection, super::raw::StaticProjection::EnumPayload(_))
                        && (loan.region.role != super::raw::RegionRole::SelectedMatchArm
                            || !(self.live_loans[at as usize] || selected && at == id))
                    {
                        return Err(reject(
                            "enum loan projection lacks live exact selected ancestry",
                        ));
                    }
                    at = parent;
                }
                super::raw::LoanOrigin::Indexed { base, .. } => match base {
                    super::raw::PlaceOrLoan::Place(place) => return self.complete(c, place),
                    super::raw::PlaceOrLoan::Loan(parent) => {
                        if !self.live_loans[parent as usize] {
                            return Err(reject("indexed parent is not live"));
                        }
                        at = parent;
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests;
