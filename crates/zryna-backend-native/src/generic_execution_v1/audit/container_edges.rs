//! Finite native helper edges independently replayed from original IR frontiers and layouts.
use super::{error, invariant};
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    CloneNodeKind, VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedCloneNode,
    VerifiedDropTarget, VerifiedOperation, raw,
};
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
use zryna_ownership_runtime_abi::LogicalOperation as O;
type Edges<'a> = BTreeMap<(&'a str, &'a str), usize>;
struct Census<'a, 'p, 's> {
    p: &'a VerifiedProgram<'p, 's>,
    imports: Vec<&'a str>,
    clones: Vec<Option<&'a str>>,
    drops: Vec<Option<&'a str>>,
    edges: Edges<'a>,
}
impl<'a> Census<'a, '_, '_> {
    fn add(&mut self, from: &'a str, to: &'a str) -> Result<(), Diagnostic> {
        let count = self.edges.entry((from, to)).or_default();
        *count = count.checked_add(1).ok_or_else(error)?;
        Ok(())
    }
    fn runtime(&mut self, from: &'a str, op: O) -> Result<(), Diagnostic> {
        let name = self
            .p
            .program()
            .runtime()
            .declarations()
            .native_linux_x86_64
            .iter()
            .find(|d| d.operation == op.name())
            .ok_or_else(invariant)?
            .symbol
            .as_str();
        if !self.imports.contains(&name) {
            return Err(error());
        }
        self.add(from, name)
    }
    fn clone(&mut self, from: &'a str, ty: u32) -> Result<(), Diagnostic> {
        self.add(from, self.clones.get(ty as usize).copied().flatten().ok_or_else(error)?)
    }
    fn drop(&mut self, from: &'a str, ty: u32) -> Result<(), Diagnostic> {
        if self.p.stored_type(ty).ok_or_else(invariant)?.drop_kind() != 0 {
            self.add(from, self.drops.get(ty as usize).copied().flatten().ok_or_else(error)?)?;
        }
        Ok(())
    }
    fn loan_drop(
        &mut self,
        from: &'a str,
        op: VerifiedOperation<'_, '_>,
    ) -> Result<(), Diagnostic> {
        for target in op.drop_targets().ok_or_else(invariant)?.targets() {
            let VerifiedDropTarget::Loan(loan) = target else {
                return Err(invariant());
            };
            self.drop(from, loan.referent())?;
        }
        Ok(())
    }
    fn cleanup(
        &mut self,
        from: &'a str,
        plan: VerifiedCleanupPlan<'_, '_>,
    ) -> Result<(), Diagnostic> {
        for action in plan.actions() {
            match action {
                VerifiedCleanupAction::EndLoan(_) => {}
                VerifiedCleanupAction::DropPlace(place) => self.drop(from, place.ty())?,
                VerifiedCleanupAction::DropInitializedPrefix { .. } => return Err(invariant()),
            }
        }
        Ok(())
    }
}
pub(super) fn edges<'a>(
    p: &'a VerifiedProgram<'_, '_>,
    imports: &[&'a str],
) -> Result<Edges<'a>, Diagnostic> {
    let plan = p.containers().ok_or_else(invariant)?;
    let mut import_names = reserve(imports.len())?;
    import_names.extend_from_slice(imports);
    let mut clones = reserve(plan.clone_symbols().len())?;
    clones.extend(plan.clone_symbols());
    let mut drops = reserve(plan.drop_symbols().len())?;
    drops.extend(plan.drop_symbols());
    let mut census = Census { p, imports: import_names, clones, drops, edges: BTreeMap::new() };
    for function in p.functions() {
        let from = function.symbol();
        for block in function.source().blocks() {
            for op in block.operations() {
                match op.kind() {
                    raw::Operation::StringFromUtf8 { .. } => {
                        census.runtime(from, O::StringFromUtf8Copy)?;
                    }
                    raw::Operation::StringConcat { .. } => census.runtime(from, O::StringConcat)?,
                    raw::Operation::VecConstruct { .. } => census.runtime(from, O::VecAllocate)?,
                    raw::Operation::VecPush { .. } => {
                        census.runtime(from, O::VecReserve)?;
                        // Checked length overflow and reserve status each have an emitted unwind.
                        census
                            .cleanup(from, op.vec_push().ok_or_else(invariant)?.outer_failure())?;
                    }
                    raw::Operation::SharedConstruct { .. } => census.runtime(from, O::Allocate)?,
                    raw::Operation::WeakDowngrade { .. } => {
                        census.runtime(from, O::WeakDowngrade)?;
                    }
                    raw::Operation::Clone { .. } => {
                        census.clone(from, op.clone_frontier().ok_or_else(invariant)?.ty())?;
                    }
                    raw::Operation::DropPlace { place }
                    | raw::Operation::ReplacePlace { place, .. } => {
                        let raw::ValueType::Stored(ty) =
                            function.place(*place).ok_or_else(invariant)?.ty()
                        else {
                            return Err(invariant());
                        };
                        let proof = op.drop_targets().ok_or_else(invariant)?;
                        if proof.is_whole() {
                            census.drop(from, ty)?;
                        } else {
                            for target in proof.targets() {
                                let ty = match target {
                                    VerifiedDropTarget::Place(p) => p.ty(),
                                    VerifiedDropTarget::Loan(l) => l.referent(),
                                };
                                census.drop(from, ty)?;
                            }
                        }
                    }
                    raw::Operation::ReplaceLoan { .. } => census.loan_drop(from, op)?,
                    _ => {}
                }
                if let Some(target) = op.call_target() {
                    census.add(
                        from,
                        p.functions().get(target.index()).ok_or_else(invariant)?.symbol(),
                    )?;
                }
                if let Some(failure) = op.failure() {
                    census.cleanup(from, failure)?;
                }
            }
            if let Some(upgrade) = block.weak_upgrade() {
                census.runtime(from, O::WeakUpgrade)?;
                census.cleanup(from, upgrade.outer_failure())?;
            }
            if let Some(exit) = block.exit_cleanup() {
                census.cleanup(from, exit)?;
            }
        }
    }
    let nodes = independent_nodes(p)?;
    for (index, node) in nodes.into_iter().enumerate() {
        if let Some(node) = node {
            let from = census.clones.get(index).copied().flatten().ok_or_else(error)?;
            clone_helper(&mut census, from, node)?;
        }
    }
    for index in 0..census.drops.len() {
        if let Some(from) = census.drops[index] {
            drop_helper(&mut census, from, u32::try_from(index).map_err(|_| error())?)?;
        }
    }
    for (export, target) in p.program().scalar_abi().exports().zip(p.program().export_functions()) {
        census.add(
            export.native_linux_x86_64_symbol().as_str(),
            p.functions().get(*target).ok_or_else(invariant)?.symbol(),
        )?;
    }
    Ok(census.edges)
}
fn clone_helper<'a>(
    c: &mut Census<'a, '_, '_>,
    from: &'a str,
    node: VerifiedCloneNode<'_, '_>,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => c.runtime(from, O::StringClone)?,
        CloneNodeKind::SharedCount => c.runtime(from, O::StrongClone)?,
        CloneNodeKind::WeakCount => c.runtime(from, O::WeakClone)?,
        CloneNodeKind::Struct => {
            for (_, child) in node.fields() {
                c.clone(from, child.ty())?;
                c.drop(from, child.ty())?;
            }
        }
        CloneNodeKind::Enum => {
            for (_, child) in node.variants() {
                if let Some(child) = child {
                    c.clone(from, child.ty())?;
                }
            }
        }
        CloneNodeKind::FixedArray { length } => {
            if length != 0 {
                let child = node.element().ok_or_else(invariant)?;
                c.clone(from, child.ty())?;
                c.drop(from, child.ty())?;
            }
        }
        CloneNodeKind::VecEach => {
            c.runtime(from, O::VecAllocate)?;
            let child = node.element().ok_or_else(invariant)?;
            c.clone(from, child.ty())?;
            c.drop(from, child.ty())?;
            c.runtime(from, O::VecReleaseStorage)?;
        }
    }
    Ok(())
}
fn drop_helper<'a>(c: &mut Census<'a, '_, '_>, from: &'a str, ty: u32) -> Result<(), Diagnostic> {
    let view = c.p.stored_type(ty).ok_or_else(invariant)?;
    match view.category() {
        TypeCategory::String => c.runtime(from, O::StringRelease)?,
        TypeCategory::Struct => {
            for (_, child, _) in view.fields() {
                c.drop(from, child.index())?;
            }
        }
        TypeCategory::Enum => {
            for (_, child) in view.variants() {
                if let Some(child) = child {
                    c.drop(from, child.index())?;
                }
            }
        }
        TypeCategory::FixedArray => {
            if view.array_length().ok_or_else(invariant)? != 0 {
                c.drop(from, view.referenced_type().ok_or_else(invariant)?.index())?;
            }
        }
        TypeCategory::Vec => {
            c.drop(from, view.referenced_type().ok_or_else(invariant)?.index())?;
            c.runtime(from, O::VecReleaseStorage)?;
        }
        TypeCategory::Shared => {
            c.runtime(from, O::StrongReleaseBegin)?;
            c.drop(from, view.referenced_type().ok_or_else(invariant)?.index())?;
            c.runtime(from, O::StrongReleaseFinish)?;
        }
        TypeCategory::Weak => c.runtime(from, O::WeakRelease)?,
        TypeCategory::Bool | TypeCategory::I32 => {}
    }
    Ok(())
}
fn independent_nodes<'p, 'a>(
    p: &VerifiedProgram<'p, 'a>,
) -> Result<Vec<Option<VerifiedCloneNode<'p, 'a>>>, Diagnostic> {
    let mut nodes = reserve(p.program().type_count())?;
    nodes.resize(p.program().type_count(), None);
    let mut queue = reserve(p.program().type_count())?;
    for op in p.program().operations() {
        if let Some(clone) = op.clone_frontier() {
            add(clone.frontier(), &mut nodes, &mut queue)?;
        }
    }
    let mut cursor = 0;
    while cursor < queue.len() {
        let node = queue[cursor];
        cursor += 1;
        for (_, child) in node.fields() {
            add(child, &mut nodes, &mut queue)?;
        }
        for (_, child) in node.variants() {
            if let Some(child) = child {
                add(child, &mut nodes, &mut queue)?;
            }
        }
        if let Some(child) = node.element() {
            add(child, &mut nodes, &mut queue)?;
        }
    }
    Ok(nodes)
}
fn add<'p, 'a>(
    node: VerifiedCloneNode<'p, 'a>,
    nodes: &mut [Option<VerifiedCloneNode<'p, 'a>>],
    queue: &mut Vec<VerifiedCloneNode<'p, 'a>>,
) -> Result<(), Diagnostic> {
    let row = nodes.get_mut(node.ty() as usize).ok_or_else(invariant)?;
    if row.is_none() {
        *row = Some(node);
        queue.push(node);
    }
    Ok(())
}

fn reserve<T>(count: usize) -> Result<Vec<T>, Diagnostic> {
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| error())?;
    Ok(result)
}
