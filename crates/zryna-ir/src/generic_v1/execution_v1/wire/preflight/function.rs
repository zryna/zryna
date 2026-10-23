//! Bounded dense definition arenas, excluding generated binding aliases.
use super::{Failure, Totals, bound, invalid, m3, raw};
fn dense(id: u32, expected: usize) -> Result<(), Failure> {
    if usize::try_from(id).map_err(|_| invalid())? == expected { Ok(()) } else { Err(invalid()) }
}
fn value(definition: &raw::ValueDefinition, seen: &mut [bool]) -> Result<(), Failure> {
    let index = usize::try_from(definition.id).map_err(|_| invalid())?;
    let slot = seen.get_mut(index).filter(|slot| !**slot).ok_or_else(invalid)?;
    *slot = true;
    Ok(())
}
pub(super) fn check(function: &raw::Function, totals: &mut Totals) -> Result<(), Failure> {
    totals.function();
    totals.parameters(function.parameters.len())?;
    totals.blocks(function.blocks.len())?;
    bound(function.places.len(), m3::MAX_PLACES_PER_FUNCTION)?;
    bound(function.loans.len(), super::super::MAX_CHILDREN)?;
    bound(function.cleanup_plans.len(), m3::MAX_CLEANUP_PLANS_PER_FUNCTION)?;
    if let Some(name) = &function.executable_export {
        bound(name.len(), 128)?;
        if !name.is_ascii() {
            return Err(invalid());
        }
    }
    let mut count = 0usize;
    for parameter in &function.parameters {
        if matches!(parameter, raw::Parameter::Value(_)) {
            totals.values(1)?;
            count = count.checked_add(1).ok_or_else(invalid)?;
        }
    }
    for (index, place) in function.places.iter().enumerate() {
        dense(place.id, index)?;
    }
    for (index, loan) in function.loans.iter().enumerate() {
        dense(loan.id, index)?;
    }
    for (index, block) in function.blocks.iter().enumerate() {
        dense(block.id, index)?;
        bound(block.value_parameters.len(), m3::MAX_BLOCK_PARAMETERS)?;
        totals.values(block.value_parameters.len())?;
        count = count.checked_add(block.value_parameters.len()).ok_or_else(invalid)?;
        totals.transitions(block.instructions.len())?;
        for instruction in &block.instructions {
            if instruction.result.is_some() {
                totals.values(1)?;
                count = count.checked_add(1).ok_or_else(invalid)?;
            }
            super::operation::check(&instruction.operation, totals)?;
        }
        terminator(&block.terminator, totals)?;
    }
    // All inherited value credit is checked before allocating the bounded uniqueness index.
    let mut seen = crate::generic_v1::reserve(count)?;
    seen.resize(count, false);
    for parameter in &function.parameters {
        if let raw::Parameter::Value(definition) = parameter {
            value(definition, &mut seen)?;
        }
    }
    for block in &function.blocks {
        for definition in &block.value_parameters {
            value(definition, &mut seen)?;
        }
        for instruction in &block.instructions {
            if let Some(definition) = &instruction.result {
                value(definition, &mut seen)?;
            }
        }
    }
    for (index, plan) in function.cleanup_plans.iter().enumerate() {
        dense(plan.id, index)?;
        totals.drops(plan.actions.len())?;
    }
    Ok(())
}
fn edge(edge: &raw::Edge, totals: &mut Totals) -> Result<(), Failure> {
    totals.edge()?;
    bound(edge.arguments.len(), m3::MAX_BLOCK_PARAMETERS)
}
fn terminator(terminator: &raw::Terminator, totals: &mut Totals) -> Result<(), Failure> {
    match terminator {
        raw::Terminator::Return { .. } | raw::Terminator::Trap { .. } => Ok(()),
        raw::Terminator::Jump(target) => edge(target, totals),
        raw::Terminator::Branch { yes, no, .. } => {
            edge(yes, totals)?;
            edge(no, totals)
        }
        raw::Terminator::ClosedEnumMatch { arms, .. } => {
            bound(arms.len(), 1024)?;
            for (index, arm) in arms.iter().enumerate() {
                dense(arm.ordinal, index)?;
                edge(&arm.edge, totals)?;
            }
            Ok(())
        }
        raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
            edge(success, totals)?;
            edge(expired, totals)
        }
    }
}
