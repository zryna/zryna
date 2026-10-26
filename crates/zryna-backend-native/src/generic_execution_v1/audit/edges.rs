//! Independent IR-frontier/cleanup-derived calls; emitter cursor/cache are not audit inputs.
use super::{error, invariant};
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    CloneNodeKind, VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedCloneNode,
    VerifiedDropTarget, VerifiedOperation, raw,
};
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
fn release_calls(p: &VerifiedProgram<'_, '_>, ty: u32) -> Result<usize, Diagnostic> {
    let view = p.stored_type(ty).ok_or_else(invariant)?;
    if view.drop_kind() == 0 {
        return Ok(0);
    }
    match view.category() {
        TypeCategory::String => Ok(1),
        TypeCategory::Struct => view.fields().try_fold(0usize, |sum, (_, child, _)| {
            sum.checked_add(release_calls(p, child.index())?).ok_or_else(error)
        }),
        TypeCategory::Enum => view.variants().try_fold(0usize, |sum, (_, child)| {
            sum.checked_add(child.map_or(Ok(0), |child| release_calls(p, child.index()))?)
                .ok_or_else(error)
        }),
        TypeCategory::FixedArray => {
            release_calls(p, view.referenced_type().ok_or_else(invariant)?.index())?
                .checked_mul(
                    usize::try_from(view.array_length().ok_or_else(invariant)?)
                        .map_err(|_| error())?,
                )
                .ok_or_else(error)
        }
        _ => Err(invariant()),
    }
}
fn clone_calls(node: VerifiedCloneNode<'_, '_>) -> Result<usize, Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => Ok(0),
        CloneNodeKind::String => Ok(1),
        CloneNodeKind::Struct => node.fields().try_fold(0usize, |sum, (_, child)| {
            sum.checked_add(clone_calls(child)?).ok_or_else(error)
        }),
        CloneNodeKind::Enum => node.variants().try_fold(0usize, |sum, (_, child)| {
            sum.checked_add(child.map_or(Ok(0), clone_calls)?).ok_or_else(error)
        }),
        CloneNodeKind::FixedArray { length } => clone_calls(node.element().ok_or_else(invariant)?)?
            .checked_mul(usize::try_from(length).map_err(|_| error())?)
            .ok_or_else(error),
        _ => Err(invariant()),
    }
}
fn cleanup_calls(
    p: &VerifiedProgram<'_, '_>,
    plan: VerifiedCleanupPlan<'_, '_>,
) -> Result<usize, Diagnostic> {
    plan.actions().try_fold(0usize, |sum, action| {
        sum.checked_add(match action {
            VerifiedCleanupAction::EndLoan(_) => 0,
            VerifiedCleanupAction::DropPlace(place) => release_calls(p, place.ty())?,
            VerifiedCleanupAction::DropInitializedPrefix { .. } => return Err(invariant()),
        })
        .ok_or_else(error)
    })
}
pub(super) fn edges<'a>(
    p: &'a VerifiedProgram<'_, '_>,
    imports: &[&'a str],
    entry: super::super::invocation::Entry,
) -> Result<BTreeMap<(&'a str, &'a str), usize>, Diagnostic> {
    if p.containers().is_some() {
        return super::container_edges::edges(p, imports, entry);
    }
    let mut edges = BTreeMap::new();
    let mut add = |from, to, n: usize| -> Result<(), Diagnostic> {
        let count = edges.entry((from, to)).or_insert(0usize);
        *count = count.checked_add(n).ok_or_else(error)?;
        Ok(())
    };
    for plan in p.functions() {
        for block in plan.source().blocks() {
            for op in block.operations() {
                match op.kind() {
                    raw::Operation::StringFromUtf8 { .. } => add(plan.symbol(), imports[0], 1)?,
                    raw::Operation::StringConcat { .. } => {
                        let symbol = p
                            .program()
                            .runtime()
                            .declarations()
                            .native_linux_x86_64
                            .iter()
                            .find(|d| d.operation == "stringConcat")
                            .ok_or_else(invariant)?
                            .symbol
                            .as_str();
                        if !imports.contains(&symbol) {
                            return Err(error());
                        }
                        add(plan.symbol(), symbol, 1)?;
                    }
                    raw::Operation::Clone { .. } => {
                        let count =
                            clone_calls(op.clone_frontier().ok_or_else(invariant)?.frontier())?;
                        add(plan.symbol(), imports[1], count)?;
                        add(plan.symbol(), imports[2], count)?;
                    }
                    raw::Operation::DropPlace { place }
                    | raw::Operation::ReplacePlace { place, .. } => {
                        let raw::ValueType::Stored(ty) =
                            plan.place(*place).ok_or_else(invariant)?.ty()
                        else {
                            return Err(invariant());
                        };
                        add(plan.symbol(), imports[2], operation_drop_calls(p, op, ty)?)?;
                    }
                    raw::Operation::ReplaceLoan { .. } => {
                        let proof = op.drop_targets().ok_or_else(invariant)?;
                        for target in proof.targets() {
                            let VerifiedDropTarget::Loan(loan) = target else {
                                return Err(invariant());
                            };
                            add(plan.symbol(), imports[2], release_calls(p, loan.referent())?)?;
                        }
                    }
                    _ => {}
                }
                if let Some(target) = op.call_target() {
                    add(
                        plan.symbol(),
                        p.functions().get(target.index()).ok_or_else(invariant)?.symbol(),
                        1,
                    )?;
                }
                if let Some(failure) = op.failure() {
                    add(plan.symbol(), imports[2], cleanup_calls(p, failure)?)?;
                }
            }
            if let Some(cleanup) = block.exit_cleanup() {
                add(plan.symbol(), imports[2], cleanup_calls(p, cleanup)?)?;
            }
        }
    }
    for (name, target) in entry.wrappers(p) {
        add(name, p.functions().get(target).ok_or_else(invariant)?.symbol(), 1)?;
    }
    Ok(edges)
}

fn operation_drop_calls(
    p: &VerifiedProgram<'_, '_>,
    op: VerifiedOperation<'_, '_>,
    whole_ty: u32,
) -> Result<usize, Diagnostic> {
    let proof = op.drop_targets().ok_or_else(invariant)?;
    if proof.is_whole() {
        return release_calls(p, whole_ty);
    }
    proof.targets().try_fold(0usize, |sum, target| {
        let ty = match target {
            VerifiedDropTarget::Place(p) => p.ty(),
            VerifiedDropTarget::Loan(l) => l.referent(),
        };
        sum.checked_add(release_calls(p, ty)?).ok_or_else(error)
    })
}
