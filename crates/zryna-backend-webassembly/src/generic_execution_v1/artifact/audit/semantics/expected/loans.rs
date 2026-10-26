//! Independent address-cell obligations from the exact retained origin, not code recipes.
use super::{Diagnostic, Event, Expr, Model, frame, place, raw, read, store};
pub(super) struct Referent {
    pub offset: u32,
    pub ty: raw::ValueType,
    pub bytes: u32,
    pub indirect: bool,
}
pub(super) fn referent(
    m: &Model<'_, '_, '_>,
    f: usize,
    source: raw::PlaceOrLoan,
) -> Result<Referent, Diagnostic> {
    match source {
        raw::PlaceOrLoan::Place(id) => {
            let slot = place(m, f, id)?;
            Ok(Referent { offset: slot.offset, ty: slot.ty, bytes: slot.bytes, indirect: false })
        }
        raw::PlaceOrLoan::Loan(id) => {
            let loan = m.loan(f, id)?;
            m.take(1)?;
            Ok(Referent {
                offset: m.loan_offset(f, id)?,
                ty: raw::ValueType::Stored(loan.referent()),
                bytes: crate::generic_execution_v1::checked_u32(
                    m.types[loan.referent() as usize].size(),
                )?,
                indirect: true,
            })
        }
    }
}
impl Referent {
    pub(super) fn address(
        &self,
        out: &mut Vec<Event>,
        delta: u32,
        m: &Model<'_, '_, '_>,
    ) -> Result<Expr, Diagnostic> {
        if self.indirect {
            read(out, frame(self.offset), 4, m)?;
            Ok(Expr::Load(0, i64::from(self.offset), 4).add(Expr::Const(i64::from(delta))))
        } else {
            Ok(frame(self.offset.checked_add(delta).ok_or_else(super::super::budget)?))
        }
    }
    pub(super) fn load(
        &self,
        out: &mut Vec<Event>,
        delta: u32,
        m: &Model<'_, '_, '_>,
    ) -> Result<Expr, Diagnostic> {
        let address = self.address(out, delta, m)?;
        read(out, address, 4, m)?;
        Ok(address.load(0, 4))
    }
}
pub(super) fn loan_end(
    out: &mut Vec<Event>,
    f: usize,
    id: u32,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    store(out, frame(m.loan_offset(f, id)?), Expr::Const(0), 4, m)
}
