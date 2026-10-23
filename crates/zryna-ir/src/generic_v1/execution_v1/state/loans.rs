use super::{Context, Failure, State, access, raw, reject, resource_failure};

impl State {
    pub(super) fn loan_room(&self) -> Result<(), Failure> {
        if self.loan_order.capacity() < self.live_loans.len()
            || self.loan_order.len() >= self.live_loans.len()
        {
            return Err(reject("loan establishment lacks prepaid authority capacity"));
        }
        Ok(())
    }
    pub(super) fn read_loan(&self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        if !self.live_loans.get(id as usize).copied().unwrap_or(false) {
            return Err(reject("loan is not live"));
        }
        self.loan_complete(c, id)?;
        if c.raw().loans[id as usize].access == raw::Access::Exclusive
            && self
                .loan_order
                .iter()
                .any(|child| c.checked().places.loans[*child as usize].parent == Some(id))
        {
            return Err(reject("exclusive parent loan is frozen by a live child"));
        }
        Ok(())
    }
    pub(super) fn begin(&mut self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        if self.live_loans[id as usize] {
            return Err(reject("BeginLoan duplicates a live loan"));
        }
        let row = &c.checked().places.loans[id as usize];
        let raw = &c.raw().loans[id as usize];
        if row.formal {
            return Err(reject("formal loan is established only by function entry"));
        }
        match raw.origin {
            raw::LoanOrigin::Place(place) => self.place(c, place, false)?,
            raw::LoanOrigin::InternalReborrow(parent) => self.read_loan(c, parent)?,
            raw::LoanOrigin::StaticProjection { parent, .. } => {
                if !self.live_loans[parent as usize] {
                    return Err(reject("projected loan parent is not live"));
                }
                self.loan_complete(c, id)?;
            }
            raw::LoanOrigin::Formal(_) => return Err(reject("formal loan cannot begin again")),
            raw::LoanOrigin::Indexed { .. } => {
                return Err(reject("indexed loan transition is not implemented yet"));
            }
        }
        for &live in &self.loan_order {
            if ancestor(c, row.parent, live) {
                continue;
            }
            if access::overlaps(c, raw::PlaceOrLoan::Loan(live), raw::PlaceOrLoan::Loan(id))
                && (raw.access == raw::Access::Exclusive
                    || c.raw().loans[live as usize].access == raw::Access::Exclusive)
            {
                return Err(reject("new loan overlaps an existing exclusive access"));
            }
        }
        if self.loan_order.len() >= crate::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION {
            return Err(resource_failure("execution peak active loans exceed inherited limit"));
        }
        self.loan_room()?;
        self.live_loans[id as usize] = true;
        self.loan_order.push(id);
        Ok(())
    }
    pub(super) fn end(&mut self, c: &Context<'_>, id: u32) -> Result<(), Failure> {
        self.read_loan(c, id)?;
        if self
            .loan_order
            .iter()
            .any(|child| c.checked().places.loans[*child as usize].parent == Some(id))
        {
            return Err(reject("cannot end a parent loan with live children"));
        }
        self.live_loans[id as usize] = false;
        let position = self
            .loan_order
            .iter()
            .position(|loan| *loan == id)
            .ok_or_else(|| reject("live loan missing from exact loan order"))?;
        self.loan_order.remove(position);
        Ok(())
    }
    pub(super) fn call_loans(
        &self,
        c: &Context<'_>,
        args: &[raw::CallArgument],
    ) -> Result<(), Failure> {
        for (i, arg) in args.iter().enumerate() {
            if let raw::CallArgument::Loan(id) = arg {
                self.read_loan(c, *id)?;
                for prior in &args[..i] {
                    if let raw::CallArgument::Loan(other) = prior
                        && access::overlaps(
                            c,
                            raw::PlaceOrLoan::Loan(*id),
                            raw::PlaceOrLoan::Loan(*other),
                        )
                        && (c.raw().loans[*id as usize].access == raw::Access::Exclusive
                            || c.raw().loans[*other as usize].access == raw::Access::Exclusive)
                    {
                        return Err(reject("call duplicates an exclusive referent alias"));
                    }
                }
            }
        }
        Ok(())
    }
}
fn ancestor(c: &Context<'_>, mut parent: Option<u32>, id: u32) -> bool {
    while let Some(at) = parent {
        if at == id {
            return true;
        }
        parent = c.checked().places.loans[at as usize].parent;
    }
    false
}
