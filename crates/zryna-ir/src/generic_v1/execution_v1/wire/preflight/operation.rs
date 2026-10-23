//! Exact inherited operation operand and literal counters.
use super::{Failure, Totals, bound, invalid, m3, raw};
pub(super) fn check(operation: &raw::Operation, totals: &mut Totals) -> Result<(), Failure> {
    match operation {
        raw::Operation::DropPlace { .. } => totals.drops(1),
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => {
            totals.call()?;
            bound(arguments.len(), m3::MAX_PARAMETERS_PER_FUNCTION)
        }
        raw::Operation::StructConstruct { fields, .. } => {
            bound(fields.len(), 1024)?;
            totals.aggregate(fields.len())
        }
        raw::Operation::ClosedEnumConstruct { payload, .. } => {
            totals.aggregate(usize::from(payload.is_some()))
        }
        raw::Operation::FixedArrayConstruct { elements, .. }
        | raw::Operation::VecConstruct { elements, .. } => totals.aggregate(elements.len()),
        raw::Operation::StringFromUtf8 { bytes, .. } => {
            totals.literal(bytes.len())?;
            std::str::from_utf8(bytes).map_err(|_| invalid())?;
            Ok(())
        }
        raw::Operation::Unit
        | raw::Operation::BoolLiteral(_)
        | raw::Operation::I32Literal(_)
        | raw::Operation::ScalarBinary { .. }
        | raw::Operation::I32Neg { .. }
        | raw::Operation::ReadPlace { .. }
        | raw::Operation::InitializePlace { .. }
        | raw::Operation::ReplacePlace { .. }
        | raw::Operation::Clone { .. }
        | raw::Operation::StringConcat { .. }
        | raw::Operation::VecPush { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. }
        | raw::Operation::BeginLoan { .. }
        | raw::Operation::EndLoan { .. }
        | raw::Operation::ReadLoan { .. }
        | raw::Operation::ReplaceLoan { .. }
        | raw::Operation::IndexCopy { .. } => Ok(()),
    }
}
