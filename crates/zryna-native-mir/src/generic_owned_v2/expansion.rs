//! Charge complete nominal destructor expansion before lane traversal or backend allocation.
use super::{budget, invariant, plan::take};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::{
    owned_v2::{VerifiedOwnedProgram, raw::Operation},
    raw,
};
use zryna_layout::generic_v1::TypeView;
mod drop_work;

pub(super) fn preflight(
    program: &VerifiedOwnedProgram<'_>,
    types: &[TypeView<'_>],
    units: &mut u64,
) -> Result<(), Diagnostic> {
    // Preserve every standard-only program's previous accounting and acceptance frontier.
    if !types.iter().any(|ty| matches!(ty.key().first(), Some(0x11 | 0x13))) {
        return Ok(());
    }
    let work = drop_work::derive(types, units)?;
    for (index, function) in program.functions().iter().enumerate() {
        let count = function.blocks.iter().try_fold(0usize, |count, block| {
            count
                .checked_add(block.parameters.len())
                .and_then(|n| n.checked_add(block.instructions.len()))
                .ok_or_else(budget)
        })?;
        if count > zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION {
            return Err(budget());
        }
        take(units, u64::try_from(count).map_err(|_| budget())?)?;
        let mut definitions = Vec::new();
        definitions.try_reserve(count).map_err(|_| budget())?;
        definitions.resize(count, None);
        for block in &function.blocks {
            for def in block.parameters.iter().chain(block.instructions.iter().map(|i| &i.result)) {
                let slot = definitions.get_mut(def.id as usize).ok_or_else(invariant)?;
                if slot.replace(def.ty).is_some() {
                    return Err(invariant());
                }
            }
        }
        let mut charge = |id: u32| {
            let ty = definitions.get(id as usize).copied().flatten().ok_or_else(invariant)?;
            let raw::Type::Stored(index) = ty else {
                return Err(invariant());
            };
            let cost = work.get(index as usize).ok_or_else(invariant)?;
            if cost.contains_nominal {
                take(units, cost.units)?;
            }
            Ok(())
        };
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Some(Operation::Drop(id)) = program.extension(index, instruction.result.id) {
                    charge(*id)?;
                }
            }
        }
        for step in &program.plan(index).ok_or_else(invariant)?.steps {
            for id in &step.cleanup {
                charge(*id)?;
            }
        }
    }
    Ok(())
}
