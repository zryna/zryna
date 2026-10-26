//! Independent exact index read, pre-child failure and post-guard address-cell obligations.
use super::super::super::{Region, cleanup, push};
use super::{
    Diagnostic, Event, Expr, Model, VerifiedOperation, frame, invariant, raw, read, referent,
    store, value,
};
use zryna_layout::TypeCategory;
pub(super) fn begin(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    m.take(8)?;
    let indexed = op.indexed_loan().ok_or_else(invariant)?;
    let function = op.site().function;
    let child = indexed.loan();
    if child.function() != function || indexed.index().function() != function {
        return Err(invariant());
    }
    let base = match (indexed.base_place(), indexed.base_loan()) {
        (Some(place), None) if place.function() == function => raw::PlaceOrLoan::Place(place.id()),
        (None, Some(loan)) if loan.function() == function => raw::PlaceOrLoan::Loan(loan.id()),
        _ => return Err(invariant()),
    };
    let source = referent(m, function, base)?;
    let container = indexed.container_type();
    if source.ty != raw::ValueType::Stored(container) {
        return Err(invariant());
    }
    let ty = *m.types.get(container as usize).ok_or_else(invariant)?;
    if ty.referenced_type().ok_or_else(invariant)?.index() != child.referent() {
        return Err(invariant());
    }
    let vector = match ty.category() {
        TypeCategory::Vec => true,
        TypeCategory::FixedArray => false,
        _ => return Err(invariant()),
    };
    let stride = if vector {
        m.element(container)?.stride()
    } else {
        ty.array_stride().ok_or_else(invariant)?
    };
    let index = value(m, function, indexed.index().id())?;
    let raw::ValueType::Stored(index_type) = index.ty else { return Err(invariant()) };
    if m.types.get(index_type as usize).ok_or_else(invariant)?.category() != TypeCategory::I32 {
        return Err(invariant());
    }
    read(out, frame(index.offset), 4, m)?;
    push(out, Event::Index(Expr::Load(0, i64::from(index.offset), 4)), m)?;
    if vector {
        source.load(out, 4, m)?;
        push(
            out,
            if source.indirect {
                Event::LoanVecBounds(0, i64::from(source.offset), 4)
            } else {
                Event::VecBounds(0, i64::from(source.offset) + 4)
            },
            m,
        )?;
    } else {
        push(
            out,
            Event::Bounds(crate::generic_execution_v1::checked_i64(
                ty.array_length().ok_or_else(invariant)?,
            )?),
            m,
        )?;
    }
    cleanup(out, op.failure().ok_or_else(invariant)?, m)?;
    push(out, Event::Status(1), m)?;
    push(out, Event::TrapReturn(1), m)?;
    push(out, Event::End(Region::Bounds), m)?;
    let address = if vector { source.load(out, 0, m)? } else { source.address(out, 0, m)? };
    let element = address.add(Expr::Scaled(5, crate::generic_execution_v1::checked_i64(stride)?));
    if element == Expr::Unknown {
        return Err(invariant());
    }
    store(out, frame(m.loan_offset(function, child.id())?), element, 4, m)
}
