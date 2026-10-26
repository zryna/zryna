//! Checked indexed loans preserve real index SSA, exact regions and parent origins.
use super::{Binding, Body, Expr, Failure, Kind, UntrustedSpan, charge, raw};
mod arguments;
impl<'s> Body<'_, 's> {
    pub(super) fn indexed_source(
        &mut self,
        mut base: Binding<'s>,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
        region: UntrustedSpan,
        mode: (raw::Access, raw::RegionRole),
    ) -> Result<Binding<'s>, Failure> {
        charge(self.spent, 1)?;
        let kind = self.world.kind(base.ty).ok_or(Failure::InternalFailure)?;
        if !matches!(kind, Kind::FixedArray { .. } | Kind::Vec) {
            return Err(self.error(span, "ZRYNA-M7006", "indexed loan requires array or Vec"));
        }
        let ty = self
            .world
            .arguments(base.ty)
            .and_then(|a| a.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        if let Kind::FixedArray { length } = kind
            && let Expr::I32Literal { spelling } =
                &self.original.body.expressions[index as usize].kind
        {
            charge(self.spent, spelling.len())?;
            if let Ok(index) = spelling.parse::<u32>()
                && index < length
            {
                base.source = self.project_source(
                    &base,
                    ty,
                    raw::StaticProjection::FixedArrayConstant(index),
                    region,
                    span,
                )?;
                if let raw::PlaceOrLoan::Loan(id) = base.source {
                    self.output.loans[id as usize].region.role = mode.1;
                }
                base.ty = ty;
                return Ok(base);
            }
        }
        self.available_target(&base, span)?;
        if mode.0 == raw::Access::Exclusive && !base.mutable {
            return Err(self.error(
                span,
                "ZRYNA-M7007",
                "indexed mutation requires exclusive source",
            ));
        }
        let value = self.expression(index, depth + 1)?;
        if value.ty != self.primitive(1)? {
            return Err(self.error(
                self.original.body.expressions[index as usize].span,
                "ZRYNA-M7006",
                "index requires exact i32",
            ));
        }
        self.available_target(&base, span)?;
        // This failure snapshot deliberately precedes establishment of the new child.
        let failure = self.failure(span)?;
        self.indexed_loan_credit()?;
        self.used[ty as usize] = true;
        let id = self.add_loan(
            ty,
            mode.0,
            raw::LoanOrigin::Indexed { base: base.source, index: value.id },
            region,
            span,
            false,
        )?;
        self.output.loans[id as usize].region.role = mode.1;
        self.instruction(
            None,
            raw::Operation::BeginLoan { loan: id, failure: Some(failure) },
            span,
        )?;
        base.source = raw::PlaceOrLoan::Loan(id);
        base.ty = ty;
        base.mutable = mode.0 == raw::Access::Exclusive;
        Ok(base)
    }
    fn indexed_loan_credit(&mut self) -> Result<(), Failure> {
        let mut cost = 0usize;
        for (count, width) in [
            (self.output.loans.len(), size_of::<raw::Loan>()),
            (self.state.loans.len(), size_of::<bool>()),
            (self.state.loan_order.len(), size_of::<u32>()),
            (self.loan_scopes.len(), size_of::<usize>()),
            (self.loan_places.len(), size_of::<Option<u32>>()),
        ] {
            cost = cost
                .checked_add(
                    count
                        .checked_add(1)
                        .and_then(|n| n.checked_mul(width + 1))
                        .ok_or(Failure::InternalFailure)?,
                )
                .ok_or(Failure::InternalFailure)?;
        }
        charge(self.spent, cost)
    }
    pub(super) fn complete_loan_source(
        &mut self,
        mut id: u32,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        loop {
            self.require_live_loan(id, span)?;
            if let Some(place) = self.loan_places[id as usize] {
                return self.complete_place(place, span);
            }
            match self.output.loans[id as usize].origin {
                raw::LoanOrigin::Formal(_) => return Ok(()),
                raw::LoanOrigin::Place(place) => return self.complete_place(place, span),
                raw::LoanOrigin::InternalReborrow(parent)
                | raw::LoanOrigin::StaticProjection { parent, .. }
                | raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Loan(parent), .. } => {
                    id = parent;
                }
                raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(place), .. } => {
                    return self.complete_place(place, span);
                }
            }
        }
    }
}
