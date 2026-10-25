//! Actual linear referent addresses; a pointer cell is never a stored Borrow or a Place.
use super::{Body, Diagnostic, I, Model, WORD, invariant, place, raw};
use zryna_ir::generic_v1::execution_v1::{VerifiedLoan, VerifiedLoanOrigin};
#[derive(Clone, Copy)]
pub(in crate::generic_execution_v1::code) struct Referent {
    pub offset: u32,
    pub ty: raw::ValueType,
    pub bytes: u32,
    pub indirect: bool,
}
impl Referent {
    pub(in crate::generic_execution_v1::code) fn address(
        self,
        b: &mut Body<'_>,
        delta: u32,
    ) -> Result<(), Diagnostic> {
        if self.indirect {
            b.load(0, self.offset)?;
            if delta != 0 {
                b.ops([
                    I::I32Const(crate::generic_execution_v1::checked_u32(delta)?.cast_signed()),
                    I::I32Add,
                ])?;
            }
            Ok(())
        } else {
            b.addr(0, self.offset.checked_add(delta).ok_or_else(super::super::super::budget)?)
        }
    }
    pub(in crate::generic_execution_v1::code) fn load(
        self,
        b: &mut Body<'_>,
        delta: u32,
    ) -> Result<(), Diagnostic> {
        self.address(b, delta)?;
        b.op(&I::I32Load(WORD))
    }
}
pub(in crate::generic_execution_v1::code) fn referent(
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
pub(in crate::generic_execution_v1::code) fn end(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    id: u32,
    f: usize,
) -> Result<(), Diagnostic> {
    b.addr(0, m.loan_offset(f, id)?)?;
    b.ops([I::I32Const(0), I::I32Store(WORD)])
}
pub(in crate::generic_execution_v1::code) fn end_view(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    loan: VerifiedLoan<'_, '_>,
) -> Result<(), Diagnostic> {
    end(m, b, loan.id(), loan.function())
}
pub(in crate::generic_execution_v1::code) fn begin(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    id: u32,
) -> Result<(), Diagnostic> {
    let loan = m.loan(f, id)?;
    b.addr(0, m.loan_offset(f, id)?)?;
    match loan.origin().ok_or_else(invariant)? {
        VerifiedLoanOrigin::Place(p) => b.addr(0, place(m, f, p.id())?.offset)?,
        VerifiedLoanOrigin::Parent(parent) => b.load(0, m.loan_offset(f, parent.id())?)?,
        VerifiedLoanOrigin::StaticProjection { parent, projection } => {
            b.load(0, m.loan_offset(f, parent.id())?)?;
            b.ops([
                I::I32Const(
                    crate::generic_execution_v1::checked_u32(
                        m.projection(parent.referent(), projection)?,
                    )?
                    .cast_signed(),
                ),
                I::I32Add,
            ])?;
        }
        VerifiedLoanOrigin::Formal(_) => return Err(invariant()),
    }
    b.op(&I::I32Store(WORD))
}
