//! New field fanout is charged before temporary formatting and descriptor retention.
use super::emission_budget;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::{VerifiedOwnedProgram, raw::Operation};
use zryna_layout::{StorageTarget, TypeCategory};

#[cfg(test)]
mod budget_tests;
#[cfg(test)]
mod source_census;
#[cfg(test)]
mod source_fixture;

pub(super) fn check(program: &VerifiedOwnedProgram<'_>) -> Result<(), Diagnostic> {
    let mut work = 0usize;
    let mut has_struct = false;
    for ty in program.layouts(StorageTarget::Linear32V1).types() {
        if ty.category() == TypeCategory::Struct {
            has_struct = true;
            charge(&mut work, ty.fields().len().checked_mul(3).ok_or_else(emission_budget)?)?;
            // Cleanup resolves each field with the retained layout iterator's nth scan.
            for (_, field, _) in ty.fields() {
                charge(
                    &mut work,
                    (field.index() as usize).checked_add(1).ok_or_else(emission_budget)?,
                )?;
            }
        }
    }
    if has_struct {
        precharge_definition_scans(program, &mut work)?;
    }
    for (fi, f) in program.functions().iter().enumerate() {
        for block in &f.blocks {
            for i in &block.instructions {
                if let Some(Operation::StructConstruct { fields, .. }) =
                    program.extension(fi, i.result.id)
                {
                    charge(&mut work, fields.len().checked_mul(3).ok_or_else(emission_budget)?)?;
                }
            }
        }
    }
    Ok(())
}

fn precharge_definition_scans(
    program: &VerifiedOwnedProgram<'_>,
    spent: &mut usize,
) -> Result<(), Diagnostic> {
    let mut work = *spent;
    for (fi, function) in program.functions().iter().enumerate() {
        let blocks = function.blocks.len();
        charge(&mut work, blocks)?;
        let (definitions, instructions) =
            function.blocks.iter().try_fold((0usize, 0usize), |(d, i), block| {
                let d = d
                    .checked_add(block.parameters.len())
                    .and_then(|n| n.checked_add(block.instructions.len()))
                    .ok_or_else(emission_budget)?;
                let i = i.checked_add(block.instructions.len()).ok_or_else(emission_budget)?;
                Ok::<_, Diagnostic>((d, i))
            })?;
        charge(&mut work, instructions)?;
        let mut lookups = 0usize;
        for block in &function.blocks {
            for instruction in &block.instructions {
                if matches!(program.extension(fi, instruction.result.id), Some(Operation::Drop(_)))
                {
                    lookups = lookups.checked_add(1).ok_or_else(emission_budget)?;
                }
            }
        }
        if let Some(plan) = program.plan(fi) {
            charge(&mut work, plan.steps.len())?;
            for step in &plan.steps {
                lookups = lookups.checked_add(step.cleanup.len()).ok_or_else(emission_budget)?;
            }
        }
        // value_type traverses block headers even when they contain no definitions.
        let scans = blocks
            .checked_add(definitions)
            .and_then(|n| n.checked_mul(lookups))
            .ok_or_else(emission_budget)?;
        charge(&mut work, scans)?;
    }
    *spent = work;
    Ok(())
}

fn charge(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    *work = work
        .checked_add(count)
        .filter(|n| *n <= crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
        .ok_or_else(emission_budget)?;
    Ok(())
}
