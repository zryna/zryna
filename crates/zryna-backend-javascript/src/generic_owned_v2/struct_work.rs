//! New field fanout is charged before temporary formatting and descriptor retention.
use super::emission_budget;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::{VerifiedOwnedProgram, raw::Operation};
use zryna_layout::{StorageTarget, TypeCategory};

pub(super) fn check(program: &VerifiedOwnedProgram<'_>) -> Result<(), Diagnostic> {
    let mut work = 0usize;
    for ty in program.layouts(StorageTarget::Linear32V1).types() {
        if ty.category() == TypeCategory::Struct {
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
fn charge(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    *work = work
        .checked_add(count)
        .filter(|n| *n <= crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES)
        .ok_or_else(emission_budget)?;
    Ok(())
}
