//! Only actual retained source Clone, drop receipts and cleanup actions seed reachability.
use super::{Diagnostic, invariant, take};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedDropTarget, VerifiedExecution,
};
fn add(id: u32, seen: &mut [bool], queue: &mut Vec<u32>) -> Result<(), Diagnostic> {
    let slot = seen.get_mut(id as usize).ok_or_else(invariant)?;
    if !*slot {
        *slot = true;
        queue.push(id);
    }
    Ok(())
}
fn cleanup(
    plan: VerifiedCleanupPlan<'_, '_>,
    seen: &mut [bool],
    queue: &mut Vec<u32>,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    for action in plan.actions() {
        take(work, 8)?;
        match action {
            VerifiedCleanupAction::EndLoan(_) => {}
            VerifiedCleanupAction::DropPlace(place)
            | VerifiedCleanupAction::DropInitializedPrefix { destination: place, .. } => {
                add(place.ty(), seen, queue)?;
            }
        }
    }
    Ok(())
}
pub(super) fn collect(
    program: &VerifiedExecution<'_>,
    seen: &mut [bool],
    queue: &mut Vec<u32>,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    for function in program.functions() {
        take(work, 8)?;
        for block in function.blocks() {
            take(work, 8)?;
            for operation in block.operations() {
                take(work, 16)?;
                if let Some(clone) = operation.clone_frontier() {
                    add(clone.ty(), seen, queue)?;
                }
                if let Some(drop) = operation.drop_targets() {
                    for target in drop.targets() {
                        take(work, 8)?;
                        add(
                            match target {
                                VerifiedDropTarget::Place(place) => place.ty(),
                                VerifiedDropTarget::Loan(loan) => loan.referent(),
                            },
                            seen,
                            queue,
                        )?;
                    }
                }
                if let Some(plan) = operation.failure() {
                    cleanup(plan, seen, queue, work)?;
                }
            }
            if let Some(plan) = block.exit_cleanup() {
                cleanup(plan, seen, queue, work)?;
            }
        }
    }
    Ok(())
}
