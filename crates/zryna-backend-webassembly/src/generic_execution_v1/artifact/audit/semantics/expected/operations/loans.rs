//! Ordered origin, exact old-drop and concat operand obligations from original seal views.
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, call, copy, failure, frame, invariant,
    place, raw, read, referent, store, value,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedDropTarget, VerifiedLoanOrigin};
mod indexed;
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    let f = op.site().function;
    match *op.kind() {
        raw::Operation::BeginLoan { loan, failure: None } => {
            let view = m.loan(f, loan)?;
            let source = match view.origin().ok_or_else(invariant)? {
                VerifiedLoanOrigin::Place(p) => frame(place(m, f, p.id())?.offset),
                VerifiedLoanOrigin::Parent(parent) => {
                    let offset = m.loan_offset(f, parent.id())?;
                    read(out, frame(offset), 4, m)?;
                    Expr::Load(0, i64::from(offset), 4)
                }
                VerifiedLoanOrigin::StaticProjection { parent, projection } => {
                    let offset = m.loan_offset(f, parent.id())?;
                    read(out, frame(offset), 4, m)?;
                    Expr::Load(0, i64::from(offset), 4)
                        .add(Expr::Const(i64::from(m.projection(parent.referent(), projection)?)))
                }
                VerifiedLoanOrigin::Formal(_) => return Err(invariant()),
            };
            store(out, frame(m.loan_offset(f, loan)?), source, 4, m)?;
        }
        raw::Operation::BeginLoan { failure: Some(_), .. } => indexed::begin(op, m, out)?,
        raw::Operation::EndLoan { loan } => super::super::loans::loan_end(out, f, loan, m)?,
        raw::Operation::ReadLoan { loan } => {
            let source = referent(m, f, raw::PlaceOrLoan::Loan(loan))?;
            let address = source.address(out, 0, m)?;
            copy(
                out,
                address,
                frame(value(m, f, op.result().ok_or_else(invariant)?.id())?.offset),
                source.bytes,
                m,
            )?;
        }
        raw::Operation::ReplaceLoan { loan, value: id } => {
            let receipt = op.drop_targets().ok_or_else(invariant)?;
            if !receipt.is_whole() {
                return Err(invariant());
            }
            let source = referent(m, f, raw::PlaceOrLoan::Loan(loan))?;
            m.take(receipt.targets().len())?;
            for target in receipt.targets() {
                let VerifiedDropTarget::Loan(target) = target else {
                    return Err(invariant());
                };
                if target.function() != f || target.id() != loan {
                    return Err(invariant());
                }
                let address = source.address(out, 0, m)?;
                call(out, m.drop_index(target.referent()), &[address], m)?;
            }
            let rhs = value(m, f, id)?;
            let address = source.address(out, 0, m)?;
            copy(out, frame(rhs.offset), address, rhs.bytes, m)?;
        }
        raw::Operation::StringConcat { left, right, .. } => {
            let target = value(m, f, op.result().ok_or_else(invariant)?.id())?;
            for offset in [0, 4, 8] {
                store(out, frame(target.offset + offset), Expr::Const(0), 4, m)?;
            }
            let left = referent(m, f, left)?.address(out, 0, m)?;
            let right = referent(m, f, right)?.address(out, 0, m)?;
            call(out, m.string_call(m.concat_index()), &[left, right, frame(target.offset)], m)?;
            failure(out, op.failure(), m)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
