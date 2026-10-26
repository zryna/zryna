//! Checked finite control guard expansion before any backend allocation.
use super::{Diagnostic, Shape, TypeCategory, VerifiedExecution, add, budget, take};
pub(in crate::generic_execution_v1::accounting) fn derive(
    program: &VerifiedExecution<'_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<u64, Diagnostic> {
    take(work, (types.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    let mut guards = 0;
    for shape in types {
        match shape.view.category() {
            TypeCategory::Shared => add(&mut guards, 3)?, // Clone, Begin, Finish.
            TypeCategory::Weak => add(&mut guards, 2)?,   // Clone, explicit Release.
            _ => {}
        }
    }
    for f in program.functions() {
        for block in f.blocks() {
            take(
                work,
                (block.operations().len() as u64)
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(128))
                    .ok_or_else(budget)?,
            )?;
            for op in block.operations() {
                if op.shared_construct().is_some() || op.weak_downgrade().is_some() {
                    add(&mut guards, 1)?;
                }
            }
            if block.weak_upgrade().is_some() {
                add(&mut guards, 1)?;
            }
        }
    }
    // Fixed scalar guards and their independent matchers emit/inspect fewer than256
    // SSA instructions/site. Birth metadata matching additionally scans actual types.len;
    // pay that genuine bound, not a per-type lane cap mistaken for a total type-count cap.
    let per_site = (types.len() as u64)
        .checked_mul(16)
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(budget)?;
    take(work, guards.checked_mul(per_site).ok_or_else(budget)?)?;
    Ok(guards)
}
