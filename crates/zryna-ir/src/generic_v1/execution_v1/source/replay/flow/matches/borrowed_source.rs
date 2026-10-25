//! Exact source role and inline parent preparation precede arm generation.
use super::{Body, Failure, UntrustedSpan, charge, raw, reject};
use crate::generic_v1::execution_v1::source::Expr;
use crate::generic_v1::execution_v1::source::replay::Binding;
#[derive(Clone, Copy)]
pub(super) struct Source {
    pub(super) parent: u32,
    pub(super) ty: u32,
    pub(super) first: usize,
}
impl Body<'_, '_> {
    pub(super) fn borrowed_scrutinee(
        &mut self,
        index: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Option<Source>, Failure> {
        charge(self.spent, 1)?;
        let first = self.output.loans.len();
        let expression = &self.original.body.expressions[index as usize];
        let target = match &expression.kind {
            Expr::Reference { .. } | Expr::FieldAccess { .. } => {
                self.source_place_in(index, depth + 1, Some(span))?
            }
            Expr::Borrow { value, .. } | Expr::BorrowMut { value, .. } => {
                let exclusive = matches!(expression.kind, Expr::BorrowMut { .. });
                let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
                let target = self.source_place_mode(
                    *value,
                    depth + 2,
                    Some(span),
                    (access, raw::RegionRole::Lexical),
                )?;
                self.available_target(&target, expression.span)?;
                if exclusive && !target.mutable {
                    return Err(self.error(
                        expression.span,
                        "ZRYNA-M7007",
                        "exclusive match requires mutable referent",
                    ));
                }
                let origin = match target.source {
                    raw::PlaceOrLoan::Place(id) => raw::LoanOrigin::Place(id),
                    raw::PlaceOrLoan::Loan(id) => raw::LoanOrigin::InternalReborrow(id),
                };
                let access = if exclusive { raw::Access::Exclusive } else { raw::Access::Shared };
                let parent =
                    self.add_loan(target.ty, access, origin, span, expression.span, true)?;
                Binding {
                    name: "",
                    source: raw::PlaceOrLoan::Loan(parent),
                    ty: target.ty,
                    mutable: exclusive,
                }
            }
            _ => return Ok(None),
        };
        let raw::PlaceOrLoan::Loan(parent) = target.source else {
            return Ok(None);
        };
        self.available_target(&target, expression.span)?;
        Ok(Some(Source { parent, ty: target.ty, first }))
    }
    pub(in crate::generic_v1::execution_v1::source::replay) fn enum_payload_child(
        &mut self,
        base: u32,
        ty: u32,
        ordinal: u32,
        span: UntrustedSpan,
    ) -> Result<u32, Failure> {
        charge(self.spent, self.children[base as usize].len() + 3)?;
        let parent = self.output.places[base as usize].ty;
        if self.world.members(parent).and_then(|m| m.get(ordinal as usize)).copied().flatten()
            != Some(ty)
        {
            return Err(reject("selected source payload differs from exact ordinal referent"));
        }
        for child in &self.children[base as usize] {
            let row = &self.output.places[*child as usize];
            if row.kind == (raw::PlaceKind::EnumPayload { base, variant: ordinal }) {
                if row.ty != ty {
                    return Err(Failure::InternalFailure);
                }
                return Ok(*child);
            }
        }
        if !matches!(
            self.world.kind(self.output.places[base as usize].ty),
            Some(
                crate::generic_v1::execution_v1::type_world::Kind::Enum { .. }
                    | crate::generic_v1::execution_v1::type_world::Kind::Option
                    | crate::generic_v1::execution_v1::type_world::Kind::Result
            )
        ) {
            return Err(reject("selected source payload requires exact closed enum head"));
        }
        charge(
            self.spent,
            (self.children[base as usize].len() + 1)
                .checked_mul(size_of::<u32>() + 1)
                .ok_or(Failure::InternalFailure)?,
        )?;
        self.children[base as usize].try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        let id = self.place(ty, raw::PlaceKind::EnumPayload { base, variant: ordinal }, span)?;
        self.children[base as usize].push(id);
        Ok(id)
    }
}
