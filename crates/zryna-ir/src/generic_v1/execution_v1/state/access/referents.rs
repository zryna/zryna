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
    if row.root.is_some() && row.local_place.is_none() {
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
        if let Some(place) = referent_place(c, id)? {
            self.complete(c, place)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
