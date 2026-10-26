//! Source-bound storage and explicit Clone roots are conservative workspace needs, not grants.
use super::{Diagnostic, TypeView, VerifiedExecution, budget, invariant, work};
use zryna_ir::generic_v1::execution_v1::{VerifiedParameter, raw::ValueType};

pub(super) fn mark(
    id: u32,
    types: &[TypeView<'_>],
    live: &mut [bool],
    queue: &mut Vec<u32>,
) -> Result<(), Diagnostic> {
    let ty = types.get(id as usize).ok_or_else(invariant)?;
    let seen = live.get_mut(id as usize).ok_or_else(invariant)?;
    if ty.drop_kind() != 0 && !*seen {
        if queue.len() >= types.len() {
            return Err(invariant());
        }
        *seen = true;
        queue.push(id);
    }
    Ok(())
}

pub(super) fn seed(
    program: &VerifiedExecution<'_>,
    types: &[TypeView<'_>],
    live: &mut [bool],
    queue: &mut Vec<u32>,
    spent: &mut usize,
) -> Result<(), Diagnostic> {
    work::take(spent, program.functions().len())?;
    let mut value = |ty: ValueType| -> Result<(), Diagnostic> {
        if let ValueType::Stored(id) = ty {
            mark(id, types, live, queue)?;
        }
        Ok(())
    };
    for function in program.functions() {
        work::take(spent, function.parameters().len().checked_mul(4).ok_or_else(budget)?)?;
        for parameter in function.parameters() {
            if let VerifiedParameter::Value(parameter) = parameter {
                value(parameter.ty())?;
            }
        }
        work::take(spent, function.places().len().checked_mul(4).ok_or_else(budget)?)?;
        for place in function.places() {
            value(ValueType::Stored(place.ty()))?;
        }
        work::take(spent, function.blocks().len())?;
        for block in function.blocks() {
            work::take(spent, block.value_parameters().len().checked_mul(4).ok_or_else(budget)?)?;
            for parameter in block.value_parameters() {
                value(parameter.ty())?;
            }
            work::take(spent, block.operations().len().checked_mul(7).ok_or_else(budget)?)?;
            for operation in block.operations() {
                if let Some(result) = operation.result() {
                    value(result.ty())?;
                }
                if let Some(clone) = operation.clone_frontier() {
                    value(ValueType::Stored(clone.ty()))?;
                }
            }
        }
    }
    Ok(())
}
