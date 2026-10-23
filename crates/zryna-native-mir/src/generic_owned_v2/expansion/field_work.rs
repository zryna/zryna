//! Complete new-field operation fanout, metered before descriptor or lane allocations.
use super::{budget, invariant, take};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::{
    owned_v2::{VerifiedOwnedProgram, raw::Operation},
    raw,
};
use zryna_layout::generic_v1::TypeView;
pub(super) fn charge(
    program: &VerifiedOwnedProgram<'_>,
    index: usize,
    function: &raw::Function,
    types: &[TypeView<'_>],
    definitions: &[Option<raw::Type>],
    units: &mut u64,
) -> Result<(), Diagnostic> {
    for block in &function.blocks {
        for instruction in &block.instructions {
            match program.extension(index, instruction.result.id) {
                Some(Operation::StructConstruct { fields, .. }) => {
                    // Existing plan::function still pays the first 256-unit instruction.
                    take(
                        units,
                        u64::try_from(fields.len().saturating_sub(1))
                            .map_err(|_| budget())?
                            .checked_mul(256)
                            .ok_or_else(budget)?,
                    )?;
                }
                Some(Operation::CopyStructField { value, .. }) => {
                    let raw::Type::Stored(ty) = definitions
                        .get(*value as usize)
                        .copied()
                        .flatten()
                        .ok_or_else(invariant)?
                    else {
                        return Err(invariant());
                    };
                    let fields = types.get(ty as usize).ok_or_else(invariant)?.fields().len();
                    // Actual backend type lookup scans definitions; field range construction
                    // visits and retains every descriptor before selecting one Copy range.
                    take(
                        units,
                        u64::try_from(definitions.len())
                            .map_err(|_| budget())?
                            .checked_add(
                                u64::try_from(fields)
                                    .map_err(|_| budget())?
                                    .checked_mul(2)
                                    .ok_or_else(budget)?,
                            )
                            .ok_or_else(budget)?,
                    )?;
                }
                Some(
                    Operation::StringLiteral(_)
                    | Operation::Move(_)
                    | Operation::Borrow { .. }
                    | Operation::EndLoan(_)
                    | Operation::Drop(_)
                    | Operation::CloneString(_)
                    | Operation::CloneBorrowedString(_),
                )
                | None => {}
            }
        }
    }
    Ok(())
}
