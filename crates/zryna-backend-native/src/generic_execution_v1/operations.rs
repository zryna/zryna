//! Same-seal native operation dispatch; physical storage is already fully prepaid.
use super::{body::Frame, invariant, records};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedDropTarget, VerifiedOperation, raw};
mod aggregate;
mod effects;
mod indexed;
mod scalars;
mod strings;
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let result = op.result().map(|value| c.value(b, value.id())).transpose()?;
    match op.kind() {
        raw::Operation::Unit | raw::Operation::EndLoan { .. } => {}
        raw::Operation::BeginLoan { .. } => {
            if let Some(view) = op.indexed_loan() {
                indexed::emit(c, b, op, view)?;
            }
        }
        raw::Operation::BoolLiteral(_)
        | raw::Operation::I32Literal(_)
        | raw::Operation::I32Neg { .. }
        | raw::Operation::ScalarBinary { .. } => scalars::emit(c, b, op, result)?,
        raw::Operation::StructConstruct { .. }
        | raw::Operation::ClosedEnumConstruct { .. }
        | raw::Operation::FixedArrayConstruct { .. } => aggregate::emit(c, b, op, result)?,
        raw::Operation::ClosedGenericCall { .. }
        | raw::Operation::SourceCall { .. }
        | raw::Operation::StringFromUtf8 { .. }
        | raw::Operation::IndexCopy { .. } => effects::emit(c, b, op, result)?,
        raw::Operation::StringConcat { .. } => strings::concat(c, b, op)?,
        raw::Operation::VecConstruct { .. }
        | raw::Operation::VecPush { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. } => super::containers::emit(c, b, op)?,
        raw::Operation::ReadPlace { place, .. } => {
            let source = c.place(b, *place)?;
            copy_result(c, b, op, source)?;
        }
        raw::Operation::ReadLoan { loan } => {
            let source = c.loan(b, *loan)?;
            copy_result(c, b, op, source)?;
        }
        raw::Operation::InitializePlace { place, value }
        | raw::Operation::ReplacePlace { place, value } => {
            let destination = c.place(b, *place)?;
            if matches!(op.kind(), raw::Operation::ReplacePlace { .. }) {
                let raw::ValueType::Stored(ty) = c.f.place(*place).ok_or_else(invariant)?.ty()
                else {
                    return Err(invariant());
                };
                drop_old(c, b, op, destination, ty)?;
            }
            let source = c.value(b, *value)?;
            records::copy(
                b,
                source,
                destination,
                c.size(c.f.value(*value).ok_or_else(invariant)?.ty())?,
            )?;
        }
        raw::Operation::ReplaceLoan { loan, value } => {
            let destination = c.loan(b, *loan)?;
            let ty = c.f.loan_type(*loan).ok_or_else(invariant)?;
            drop_old(c, b, op, destination, ty)?;
            let source = c.value(b, *value)?;
            records::copy(b, source, destination, c.size(raw::ValueType::Stored(ty))?)?;
        }
        raw::Operation::DropPlace { place } => {
            let destination = c.place(b, *place)?;
            let raw::ValueType::Stored(ty) = c.f.place(*place).ok_or_else(invariant)?.ty() else {
                return Err(invariant());
            };
            drop_old(c, b, op, destination, ty)?;
        }
        raw::Operation::Clone { .. } => {
            super::clone::emit(c, b, op)?;
        }
    }
    Ok(())
}
pub(super) fn source_type(
    c: &Frame<'_, '_, '_>,
    source: raw::PlaceOrLoan,
) -> Result<u32, Diagnostic> {
    match source {
        raw::PlaceOrLoan::Place(id) => match c.f.place(id).ok_or_else(invariant)?.ty() {
            raw::ValueType::Stored(ty) => Ok(ty),
            raw::ValueType::Unit => Err(invariant()),
        },
        raw::PlaceOrLoan::Loan(id) => c.f.loan_type(id).ok_or_else(invariant),
    }
}
pub(super) fn scalar(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    id: u32,
) -> Result<Value, Diagnostic> {
    let record = c.f.value(id).ok_or_else(invariant)?;
    let address = c.record(b, record);
    match c.size(record.ty())? {
        1 => {
            let value = b.ins().load(types::I8, MemFlagsData::new(), address, 0);
            Ok(b.ins().uextend(types::I32, value))
        }
        4 => Ok(b.ins().load(types::I32, MemFlagsData::new(), address, 0)),
        _ => Err(invariant()),
    }
}
fn copy_result(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    source: Value,
) -> Result<(), Diagnostic> {
    let result = op.result().ok_or_else(invariant)?;
    let destination = c.value(b, result.id())?;
    records::copy(b, source, destination, c.size(result.ty())?)
}

fn drop_old(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    whole: Value,
    ty: u32,
) -> Result<(), Diagnostic> {
    let proof = op.drop_targets().ok_or_else(invariant)?;
    if proof.is_whole() {
        return records::drop_value(c, b, ty, whole);
    }
    for target in proof.targets() {
        let (address, ty) = match target {
            VerifiedDropTarget::Place(p) => (c.place(b, p.id())?, p.ty()),
            VerifiedDropTarget::Loan(l) => (c.loan(b, l.id())?, l.referent()),
        };
        records::drop_value(c, b, ty, address)?;
    }
    Ok(())
}
