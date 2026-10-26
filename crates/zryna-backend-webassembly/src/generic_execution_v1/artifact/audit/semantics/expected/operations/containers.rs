//! Independently derived acquisition, pending cleanup and record commitment obligations.
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, call, copy, failure, invariant, read, store,
    value,
};
use crate::generic_execution_v1::model::containers as C;
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let dst = value(m, f, op.result().ok_or_else(invariant)?.id())?;
    if let Some(v) = op.vec_construct() {
        let meta = m.element(v.ty())?;
        call(
            out,
            m.container(C::VEC_ALLOCATE),
            &[
                Expr::Const(i64::from(meta.element().index())),
                Expr::Const(crate::generic_execution_v1::checked_i64(v.elements().len())?),
                Expr::address(0, i64::from(dst.offset)),
            ],
            m,
        )?;
        failure(out, Some(v.outer_failure()), m)?;
        read(out, Expr::address(0, i64::from(dst.offset)), 4, m)?;
        m.take(v.elements().len())?;
        for (index, item) in v.elements().enumerate() {
            let source = value(m, f, item.id())?;
            let target = if index == 0 {
                Expr::Load(0, i64::from(dst.offset), 4)
            } else {
                Expr::Deref(
                    0,
                    i64::from(dst.offset),
                    crate::generic_execution_v1::checked_i64(index as u64 * meta.stride())?,
                )
            };
            copy(out, Expr::address(0, i64::from(source.offset)), target, source.bytes, m)?;
        }
        store(
            out,
            Expr::address(0, i64::from(dst.offset) + 4),
            Expr::Const(crate::generic_execution_v1::checked_i64(v.elements().len())?),
            4,
            m,
        )?;
    } else if let Some(v) = op.shared_construct() {
        shared(v, op, m, out)?;
    } else if let Some(v) = op.weak_downgrade() {
        let source = if let Some(p) = v.source_place() {
            super::raw::PlaceOrLoan::Place(p.id())
        } else {
            super::raw::PlaceOrLoan::Loan(v.source_loan().ok_or_else(invariant)?.id())
        };
        let source = super::referent(m, f, source)?;
        let pointer = source.load(out, 0, m)?;
        call(out, m.container(C::DOWNGRADE), &[pointer], m)?;
        failure(out, Some(v.outer_failure()), m)?;
        let pointer = source.load(out, 0, m)?;
        store(out, Expr::address(0, i64::from(dst.offset)), pointer, 4, m)?;
    } else {
        return Err(invariant());
    }
    Ok(())
}

fn shared(
    v: zryna_ir::generic_v1::execution_v1::VerifiedSharedConstruct<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    let dst = value(m, f, op.result().ok_or_else(invariant)?.id())?;

    let meta = m.control(v.ty())?;
    let scratch = i64::from(m.memory.claims().scratch_start);
    call(
        out,
        m.control_birth(),
        &[
            Expr::Const(crate::generic_execution_v1::checked_i64(meta.size())?),
            Expr::Const(crate::generic_execution_v1::checked_i64(meta.alignment())?),
        ],
        m,
    )?;
    store(out, Expr::Const(scratch), Expr::PackedCallResult(m.control_birth()), 8, m)?;
    read(out, Expr::Const(scratch + 4), 4, m)?;
    failure(out, Some(v.outer_failure()), m)?;
    read(out, Expr::Const(scratch), 4, m)?;
    for offset in [0, 4] {
        store(
            out,
            if offset == 0 {
                Expr::AbsoluteLoad(scratch, 4)
            } else {
                Expr::AbsoluteDeref(scratch, offset)
            },
            Expr::Const(1),
            4,
            m,
        )?;
    }
    let source = value(m, f, v.payload().id())?;
    copy(
        out,
        Expr::address(0, i64::from(source.offset)),
        Expr::AbsoluteDeref(
            scratch,
            crate::generic_execution_v1::checked_i64(meta.payload_offset())?,
        ),
        source.bytes,
        m,
    )?;
    store(out, Expr::address(0, i64::from(dst.offset)), Expr::AbsoluteLoad(scratch, 4), 4, m)?;

    Ok(())
}
