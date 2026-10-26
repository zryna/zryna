//! Exact append referent/pending/RHS/control obligations from retained source views.
use super::{
    Diagnostic, Event, Expr, Model, Region, VerifiedOperation, call, cleanup, copy, failure, frame,
    invariant, push, raw, referent, store, value,
};
use crate::generic_execution_v1::model::containers as C;
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let append = op.vec_push().ok_or_else(invariant)?;
    let f = op.site().function;
    let source = if let Some(p) = append.target_place() {
        raw::PlaceOrLoan::Place(p.id())
    } else {
        raw::PlaceOrLoan::Loan(append.target_loan().ok_or_else(invariant)?.id())
    };
    let source = referent(m, f, source)?;
    let pending = m.pending(op.site())?;
    let rhs = value(m, f, append.value().id())?;
    let metadata = m.element(append.ty())?;
    source.load(out, 4, m)?;
    push(
        out,
        if source.indirect {
            Event::LoanAppendFull(0, i64::from(source.offset), 4)
        } else {
            Event::AppendFull(i64::from(source.offset) + 4)
        },
        m,
    )?;
    cleanup(out, append.outer_failure(), m)?;
    push(out, Event::Status(3), m)?;
    push(out, Event::TrapReturn(3), m)?;
    push(out, Event::End(Region::AppendFull), m)?;
    for offset in [0, 4, 8] {
        store(out, frame(pending.offset + offset), Expr::Const(0), 4, m)?;
    }
    let address = source.address(out, 0, m)?;
    let length = source.load(out, 4, m)?;
    call(
        out,
        m.container(C::VEC_RESERVE),
        &[
            Expr::Const(i64::from(metadata.element().index())),
            address,
            length.add(Expr::Const(1)),
            frame(pending.offset),
        ],
        m,
    )?;
    failure(out, Some(append.outer_failure()), m)?;
    super::read(out, frame(pending.offset), 4, m)?;
    let length = source.load(out, 4, m)?;
    let stride =
        i64::from(crate::generic_execution_v1::checked_u32(metadata.stride())?.cast_signed());
    let destination =
        Expr::Load(0, i64::from(pending.offset), 4).add(length.mul(Expr::Const(stride)));
    copy(out, frame(rhs.offset), destination, rhs.bytes, m)?;
    let length = source.load(out, 4, m)?;
    store(out, frame(pending.offset + 4), length.add(Expr::Const(1)), 4, m)?;
    let address = source.address(out, 0, m)?;
    copy(out, frame(pending.offset), address, pending.bytes, m)
}
