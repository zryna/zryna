//! Indexed Begin publishes its exact address cell only after retained bounds failure has exited.
use super::{Body, Diagnostic, I, Model, VerifiedOperation, WORD, invariant, raw, s};
use wasm_encoder::BlockType;
use zryna_layout::TypeCategory;
pub(super) fn begin(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    // Fixed opaque view/base/type/index checks are paid independently of instruction emission.
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
    let source = s::referent(m, function, base)?;
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
    let index = s::value(m, function, indexed.index().id())?;
    let raw::ValueType::Stored(index_type) = index.ty else { return Err(invariant()) };
    if m.types.get(index_type as usize).ok_or_else(invariant)?.category() != TypeCategory::I32 {
        return Err(invariant());
    }
    s::scalar(b, index)?;
    b.op(&I::LocalTee(5))?;
    if vector {
        source.load(b, 4)?;
    } else {
        b.op(&I::I32Const(
            crate::generic_execution_v1::checked_u32(ty.array_length().ok_or_else(invariant)?)?
                .cast_signed(),
        ))?;
    }
    b.ops([I::I32GeU, I::If(BlockType::Empty)])?;
    s::cleanup(m, b, op.failure().ok_or_else(invariant)?)?;
    b.ops([I::I32Const(1), I::GlobalSet(1), I::I32Const(1), I::Return, I::End])?;
    // No child pointer store occurs in the failing arm or before its bounds guard.
    b.addr(0, m.loan_offset(function, child.id())?)?;
    if vector {
        source.load(b, 0)?;
    } else {
        source.address(b, 0)?;
    }
    b.ops([
        I::LocalGet(5),
        I::I32Const(crate::generic_execution_v1::checked_u32(stride)?.cast_signed()),
        I::I32Mul,
        I::I32Add,
        I::I32Store(WORD),
    ])
}
