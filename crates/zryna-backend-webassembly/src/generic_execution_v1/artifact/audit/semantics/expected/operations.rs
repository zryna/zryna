//! Checked vocabulary dispatch for independent source operation obligations.
use super::{
    Diagnostic, Event, Expr, Model, Region, call, cleanup, copy, drop, failure, frame, invariant,
    place, push, raw, read, referent, store, value,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, VerifiedParameter};
mod calls;
mod construct;
mod containers;
mod index;
mod places;
mod scalar;
pub(super) fn emit(
    op: VerifiedOperation<'_, '_>,
    m: &Model<'_, '_, '_>,
    out: &mut Vec<Event>,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::Unit
        | raw::Operation::I32Literal(_)
        | raw::Operation::BoolLiteral(_)
        | raw::Operation::ScalarBinary { .. }
        | raw::Operation::I32Neg { .. } => scalar::emit(op, m, out),
        raw::Operation::ReadPlace { .. }
        | raw::Operation::InitializePlace { .. }
        | raw::Operation::ReplacePlace { .. }
        | raw::Operation::DropPlace { .. } => places::emit(op, m, out),
        raw::Operation::Clone { .. }
        | raw::Operation::StringFromUtf8 { .. }
        | raw::Operation::SourceCall { .. }
        | raw::Operation::ClosedGenericCall { .. } => calls::emit(op, m, out),
        raw::Operation::StructConstruct { .. }
        | raw::Operation::ClosedEnumConstruct { .. }
        | raw::Operation::FixedArrayConstruct { .. } => construct::emit(op, m, out),
        raw::Operation::VecConstruct { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. } => containers::emit(op, m, out),
        raw::Operation::IndexCopy { .. } => index::emit(op, m, out),
        _ => Err(invariant()),
    }
}
