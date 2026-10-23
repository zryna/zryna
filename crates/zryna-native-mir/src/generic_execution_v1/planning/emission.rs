//! Complete emission, exit and failure work preflight.
use super::super::containers::Extensions;
use super::{CopySite, Diagnostic, Record, Shape, budget, copy_cost, cost, invariant, take};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedDropTarget, VerifiedFunction,
    VerifiedOperation, raw,
};
pub(super) fn check(
    f: VerifiedFunction<'_, '_>,
    types: &[Shape<'_>],
    values: &[Record],
    places: &[Record],
    work: &mut u64,
) -> Result<Extensions, Diagnostic> {
    let mut extensions = Extensions::default();
    for b in f.blocks() {
        for op in b.operations() {
            extensions.push |= op.vec_push().is_some();
            extensions.concat |= matches!(op.kind(), raw::Operation::StringConcat { .. });
            if let Some(plan) = op.failure() {
                cleanup(plan, types, work)?;
            }
            operation(op, values, places, types, work)?;
        }
        if let Some(plan) = b.exit_cleanup() {
            cleanup(plan, types, work)?;
        }
        if let Some(upgrade) = b.weak_upgrade() {
            extensions.upgrade = true;
            take(work, 512)?;
            cleanup(upgrade.outer_failure(), types, work)?;
            cost(types, upgrade.success_binding().ty(), work)?;
        }
        terminator(f, b.terminator(), values, types, work)?;
    }
    Ok(extensions)
}
fn operation(
    op: VerifiedOperation<'_, '_>,
    values: &[Record],
    places: &[Record],
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::StringFromUtf8 { bytes, .. } => {
            take(work, (bytes.len() as u64).checked_mul(64).ok_or_else(budget)?)?;
        }
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => {
            if op.call_target().is_none() {
                return Err(invariant());
            }
            take(work, (arguments.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
        }
        raw::Operation::InitializePlace { value, .. }
        | raw::Operation::ReplacePlace { value, .. } => {
            copy_cost(
                CopySite::Initialize,
                types,
                values.get(*value as usize).ok_or_else(invariant)?.ty,
                work,
            )?;
            if let raw::Operation::ReplacePlace { place, .. } = op.kind() {
                old_drop(op, *place, places, types, work)?;
            }
        }
        raw::Operation::DropPlace { place } => {
            old_drop(op, *place, places, types, work)?;
        }
        raw::Operation::StructConstruct { fields, .. } => {
            take(work, (fields.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
        }
        raw::Operation::FixedArrayConstruct { elements, .. } => {
            take(work, (elements.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
        }
        raw::Operation::VecConstruct { .. } => {
            let view = op.vec_construct().ok_or_else(invariant)?;
            take(work, (view.elements().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
            for value in view.elements() {
                cost(types, value.ty(), work)?;
            }
        }
        raw::Operation::SharedConstruct { .. } => {
            cost(types, op.shared_construct().ok_or_else(invariant)?.payload().ty(), work)?;
        }
        raw::Operation::VecPush { .. } => {
            let view = op.vec_push().ok_or_else(invariant)?;
            take(work, 512)?;
            cleanup(view.outer_failure(), types, work)?;
            cost(types, view.value().ty(), work)?;
        }
        raw::Operation::ReplaceLoan { value, .. } => {
            let ty = values.get(*value as usize).ok_or_else(invariant)?.ty;
            copy_cost(CopySite::Initialize, types, ty, work)?;
            let proof = op.drop_targets().ok_or_else(invariant)?;
            take(work, (proof.targets().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
            for target in proof.targets() {
                let VerifiedDropTarget::Loan(loan) = target else {
                    return Err(invariant());
                };
                cost(types, raw::ValueType::Stored(loan.referent()), work)?;
            }
        }
        raw::Operation::StringConcat { .. } => take(work, 512)?,
        _ => {}
    }
    Ok(())
}
fn terminator(
    f: VerifiedFunction<'_, '_>,
    term: &raw::Terminator,
    values: &[Record],
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    match term {
        raw::Terminator::Return { .. } => {
            copy_cost(CopySite::Return, types, f.result_type(), work)?;
        }
        raw::Terminator::Jump(edge) => edge_cost(edge, values, types, work)?,
        raw::Terminator::Branch { yes, no, .. } => {
            edge_cost(yes, values, types, work)?;
            edge_cost(no, values, types, work)?;
        }
        raw::Terminator::ClosedEnumMatch { ty, mode, arms, .. } => {
            if *mode != raw::MatchMode::Value {
                return Err(invariant());
            }
            cost(types, raw::ValueType::Stored(*ty), work)?;
            take(work, (arms.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
            for arm in arms {
                edge_cost(&arm.edge, values, types, work)?;
            }
        }
        raw::Terminator::WeakUpgradeBranch { success, expired, .. } => {
            edge_cost(success, values, types, work)?;
            edge_cost(expired, values, types, work)?;
        }
        raw::Terminator::Trap { .. } => {}
    }
    Ok(())
}

fn edge_cost(
    edge: &raw::Edge,
    values: &[Record],
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    take(work, (edge.arguments.len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    for id in &edge.arguments {
        cost(types, values.get(*id as usize).ok_or_else(invariant)?.ty, work)?;
    }
    Ok(())
}
fn cleanup(
    plan: VerifiedCleanupPlan<'_, '_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    take(work, (plan.actions().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    for action in plan.actions() {
        match action {
            VerifiedCleanupAction::DropPlace(place) => {
                cost(types, raw::ValueType::Stored(place.ty()), work)?;
            }
            VerifiedCleanupAction::EndLoan(_) => {}
            VerifiedCleanupAction::DropInitializedPrefix { .. } => return Err(invariant()),
        }
    }
    Ok(())
}

fn old_drop(
    op: VerifiedOperation<'_, '_>,
    place: u32,
    places: &[Record],
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    let proof = op.drop_targets().ok_or_else(invariant)?;
    if proof.is_whole() {
        return cost(types, places.get(place as usize).ok_or_else(invariant)?.ty, work);
    }
    take(work, (proof.targets().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    for target in proof.targets() {
        let ty = match target {
            VerifiedDropTarget::Place(p) => p.ty(),
            VerifiedDropTarget::Loan(l) => l.referent(),
        };
        cost(types, raw::ValueType::Stored(ty), work)?;
    }
    Ok(())
}
