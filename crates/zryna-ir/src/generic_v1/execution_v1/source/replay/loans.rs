//! Source-derived exact loan rows, never SSA Borrow values or caller-selected authority.
use super::{Binding, Body, Expr, Failure, UntrustedSpan, Value, charge, lookup, raw, reject};
mod bindings;
mod scopes;
impl<'s> Body<'_, 's> {
    pub(super) fn loan_referent(&mut self, key: &[u8]) -> Result<u32, Failure> {
        let id = lookup(&self.program.type_keys, key, self.spent)?;
        self.used[id as usize] = true;
        Ok(id)
    }
    pub(super) fn add_loan(
        &mut self,
        referent: u32,
        access: raw::Access,
        origin: raw::LoanOrigin,
        region: UntrustedSpan,
        site: UntrustedSpan,
        begin: bool,
    ) -> Result<u32, Failure> {
        let backing = match origin {
            raw::LoanOrigin::Place(id) => Some(id),
            raw::LoanOrigin::InternalReborrow(id) => self.loan_places[id as usize],
            raw::LoanOrigin::StaticProjection { parent, projection } => {
                if let Some(base) = self.loan_places[parent as usize] {
                    Some(if let raw::StaticProjection::EnumPayload(ordinal) = projection {
                        self.enum_payload_child(base, referent, ordinal, site)?
                    } else {
                        self.static_child(base, referent, projection, site)?
                    })
                } else {
                    None
                }
            }
            raw::LoanOrigin::Formal(_) | raw::LoanOrigin::Indexed { .. } => None,
        };
        charge(self.spent, 6)?;
        let id = u32::try_from(self.output.loans.len()).map_err(|_| Failure::InternalFailure)?;
        self.output.loans.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.loans.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.loan_order.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.loan_scopes.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.loan_places.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.output.loans.push(raw::Loan {
            id,
            referent,
            access,
            region: raw::Region { span: region, role: raw::RegionRole::Lexical },
            origin,
        });
        self.state.loans.push(true);
        self.state.loan_order.push(id);
        self.loan_scopes.push(self.state.locals.len());
        self.loan_places.push(backing);
        if begin {
            self.instruction(None, raw::Operation::BeginLoan { loan: id, failure: None }, site)?;
        }
        Ok(id)
    }
    pub(super) fn bind_loan(
        &mut self,
        name: &'s str,
        id: u32,
        ty: u32,
        exclusive: bool,
    ) -> Result<(), Failure> {
        charge(self.spent, 1)?;
        self.state.locals.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        self.state.locals.push(Binding {
            name,
            source: raw::PlaceOrLoan::Loan(id),
            ty,
            mutable: exclusive,
        });
        Ok(())
    }
    pub(super) fn project_source(
        &mut self,
        base: &Binding<'_>,
        ty: u32,
        projection: raw::StaticProjection,
        region: UntrustedSpan,
        site: UntrustedSpan,
    ) -> Result<raw::PlaceOrLoan, Failure> {
        match base.source {
            raw::PlaceOrLoan::Place(id) => {
                self.static_child(id, ty, projection, site).map(raw::PlaceOrLoan::Place)
            }
            raw::PlaceOrLoan::Loan(parent) => {
                self.require_live_loan(parent, site)?;
                let access = self.output.loans[parent as usize].access;
                self.add_loan(
                    ty,
                    access,
                    raw::LoanOrigin::StaticProjection { parent, projection },
                    region,
                    site,
                    true,
                )
                .map(raw::PlaceOrLoan::Loan)
            }
        }
    }
    pub(super) fn read_source(
        &mut self,
        b: &Binding<'_>,
        span: UntrustedSpan,
    ) -> Result<Value, Failure> {
        self.available_target(b, span)?;
        let copy = self.copy(b.ty)?;
        let operation = match b.source {
            raw::PlaceOrLoan::Place(id) => {
                if !copy {
                    return self.move_read(id, b.ty, span);
                }
                raw::Operation::ReadPlace { place: id, mode: raw::ReadMode::Copy }
            }
            raw::PlaceOrLoan::Loan(id) => {
                if !copy {
                    return Err(self.error(
                        span,
                        "ZRYNA-M7007",
                        "borrowed affine referent cannot move implicitly",
                    ));
                }
                raw::Operation::ReadLoan { loan: id }
            }
        };
        self.emit(raw::ValueType::Stored(b.ty), operation, span)
    }
    pub(super) fn drop_observed(
        &mut self,
        b: &Binding<'_>,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let raw::PlaceOrLoan::Place(id) = b.source else {
            return Err(reject("temporary observation has no exact owned place"));
        };
        self.drop_place(id, span)
    }
}
