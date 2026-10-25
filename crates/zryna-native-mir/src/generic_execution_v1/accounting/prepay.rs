//! Charge the actual finite emitted leaf expansion before backend allocations/code generation.
use super::{
    Diagnostic, Shape, TypeCategory, VerifiedCleanupAction, VerifiedExecution, VerifiedOperation,
    budget, invariant, raw, take,
};
mod control;
pub(super) use control::derive as controls;
fn add(total: &mut u64, n: u64) -> Result<(), Diagnostic> {
    *total = total.checked_add(n).ok_or_else(budget)?;
    Ok(())
}
fn strings(types: &[Shape<'_>], ty: u32) -> Result<u64, Diagnostic> {
    Ok(types.get(ty as usize).ok_or_else(invariant)?.strings)
}
fn cleanup(
    plan: zryna_ir::generic_v1::execution_v1::VerifiedCleanupPlan<'_, '_>,
    types: &[Shape<'_>],
    helpers: bool,
    guards: &mut u64,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    take(work, (plan.actions().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    if helpers {
        return Ok(());
    }
    for action in plan.actions() {
        if let VerifiedCleanupAction::DropPlace(p) = action {
            add(guards, strings(types, p.ty())?)?;
        }
    }
    Ok(())
}
pub(super) fn derive(
    program: &VerifiedExecution<'_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<u64, Diagnostic> {
    take(work, (types.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    let helpers = types.iter().any(|s| {
        matches!(s.view.category(), TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak)
    });
    let mut guards = 0;
    take(work, (program.scalar_abi().exports().len() as u64).checked_mul(512).ok_or_else(budget)?)?;
    for f in program.functions() {
        take(work, 256)?;
        for b in f.blocks() {
            take(
                work,
                (b.operations().len() as u64)
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(128))
                    .ok_or_else(budget)?,
            )?;
            for op in b.operations() {
                operation(f, op, types, helpers, &mut guards, work)?;
            }
            if let Some(plan) = b.exit_cleanup() {
                cleanup(plan, types, helpers, &mut guards, work)?;
            }
            if let Some(upgrade) = b.weak_upgrade() {
                cleanup(upgrade.outer_failure(), types, helpers, &mut guards, work)?;
            }
        }
    }
    if helpers {
        // String helper bodies each have at most one acquisition and one release leaf.
        // Other aggregate helpers forward context and dispatch only actual active children.
        for shape in types {
            if shape.view.category() == TypeCategory::String {
                add(&mut guards, 2)?;
            }
            let edges = (shape.view.fields().len() as u64)
                .checked_add(shape.view.variants().len() as u64)
                .and_then(|n| n.checked_add(4))
                .ok_or_else(budget)?;
            take(work, edges.checked_mul(256).ok_or_else(budget)?)?;
        }
    }
    // Each emitted leaf has fewer than128 generated SSA instructions and a fixed-size
    // independent matcher. This2048 allowance pays emission, inspection and context transport.
    take(work, guards.checked_mul(2048).ok_or_else(budget)?)?;
    Ok(guards)
}
fn operation(
    f: zryna_ir::generic_v1::execution_v1::VerifiedFunction<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    types: &[Shape<'_>],
    helpers: bool,
    guards: &mut u64,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::StringFromUtf8 { .. } | raw::Operation::StringConcat { .. } => {
            add(guards, 1)?;
        }
        raw::Operation::Clone { .. } if !helpers => add(
            guards,
            strings(types, op.clone_frontier().ok_or_else(invariant)?.ty())?
                .checked_mul(2)
                .ok_or_else(budget)?,
        )?,
        raw::Operation::DropPlace { .. }
        | raw::Operation::ReplacePlace { .. }
        | raw::Operation::ReplaceLoan { .. }
            if !helpers =>
        {
            let proof = op.drop_targets().ok_or_else(invariant)?;
            if proof.is_whole() {
                let ty = match op.kind() {
                    raw::Operation::DropPlace { place }
                    | raw::Operation::ReplacePlace { place, .. } => {
                        f.places().nth(*place as usize).ok_or_else(invariant)?.ty()
                    }
                    raw::Operation::ReplaceLoan { loan, .. } => {
                        f.loans().nth(*loan as usize).ok_or_else(invariant)?.referent()
                    }
                    _ => return Err(invariant()),
                };
                add(guards, strings(types, ty)?)?;
            } else {
                take(work, (proof.targets().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
                for target in proof.targets() {
                    let ty = match target {
                        zryna_ir::generic_v1::execution_v1::VerifiedDropTarget::Place(p) => p.ty(),
                        zryna_ir::generic_v1::execution_v1::VerifiedDropTarget::Loan(l) => {
                            l.referent()
                        }
                    };
                    add(guards, strings(types, ty)?)?;
                }
            }
        }
        _ => {}
    }
    if let Some(plan) = op.failure() {
        cleanup(plan, types, helpers, guards, work)?;
    }

    Ok(())
}

// The earlier derive walk already pays each type, source block and operation census.
// Each Vec helper has at most clone Allocate, clone rollback storage Release and drop Release.
// Direct source construction/push contributes one independently guarded outer canonical call.
pub(super) fn vectors(
    program: &VerifiedExecution<'_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<u64, Diagnostic> {
    take(work, (types.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    let mut guards = 0;
    for shape in types {
        if shape.view.category() == TypeCategory::Vec {
            add(&mut guards, 3)?;
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
                if op.vec_construct().is_some() || op.vec_push().is_some() {
                    add(&mut guards, 1)?;
                }
            }
        }
    }
    // A fixed20-step canonical capacity classifier emits fewer than512 SSA instructions.
    // This8192 pays emission, independent instruction matching and context transport
    // before any backend allocation/code generation. Element metadata lookup is O(1)
    // through the already allocated/paid elements_by_child table retained by ContainerPlan.
    take(work, guards.checked_mul(8192).ok_or_else(budget)?)?;
    Ok(guards)
}
