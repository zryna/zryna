//! Counted and prepaid discovery; None variants and duplicate roots still consume visits.
use super::{
    Diagnostic, StorageTarget, TypeView, VerifiedCloneNode, VerifiedExecution, VerifiedFunction,
    budget, invariant, work,
};
type Inventory<'p, 'a> =
    (Vec<TypeView<'a>>, Vec<VerifiedFunction<'p, 'a>>, Vec<Option<VerifiedCloneNode<'p, 'a>>>);
pub(super) fn collect<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    spent: &mut usize,
) -> Result<Inventory<'p, 'a>, Diagnostic> {
    let layout = program.layouts(StorageTarget::Linear32V1);
    work::take(spent, layout.types().len())?;
    work::retained::<TypeView<'a>>(spent, layout.types().len())?;
    let mut types = Vec::new();
    #[cfg(test)]
    work::reserve_seen(layout.types().len());
    types.try_reserve_exact(layout.types().len()).map_err(|_| budget())?;
    for ty in layout.types() {
        if ty.array_length().is_some_and(|n| u32::try_from(n).is_err()) {
            return Err(invariant());
        }
        types.push(ty);
    }
    work::retained::<VerifiedFunction<'p, 'a>>(spent, program.functions().len())?;
    let mut functions = Vec::new();
    functions.try_reserve_exact(program.functions().len()).map_err(|_| budget())?;
    functions.extend(program.functions());
    work::retained::<Option<VerifiedCloneNode<'p, 'a>>>(spent, types.len())?;
    let mut clones = Vec::new();
    clones.try_reserve_exact(types.len()).map_err(|_| budget())?;
    clones.resize(types.len(), None);
    let mut stack = Vec::new();
    work::take(spent, functions.len())?;
    for function in &functions {
        work::take(spent, function.loans().len())?;
        if function.loans().any(|loan| loan.origin().is_none() && loan.indexed().is_none()) {
            return Err(invariant());
        }
        work::take(spent, function.blocks().len())?;
        for block in function.blocks() {
            work::take(spent, block.operations().len())?;
            for op in block.operations() {
                if let Some(clone) = op.clone_frontier() {
                    work::retained::<VerifiedCloneNode<'p, 'a>>(spent, 1)?;
                    stack.try_reserve(1).map_err(|_| budget())?;
                    stack.push(clone.frontier());
                }
            }
        }
    }
    while let Some(node) = stack.pop() {
        work::take(spent, 1)?;
        let index = node.ty() as usize;
        if clones.get(index).ok_or_else(invariant)?.is_some() {
            continue;
        }
        clones[index] = Some(node);
        expand(node, &mut stack, spent)?;
    }
    Ok((types, functions, clones))
}

#[cfg(test)]
mod tests;

fn expand<'p, 'a>(
    node: VerifiedCloneNode<'p, 'a>,
    stack: &mut Vec<VerifiedCloneNode<'p, 'a>>,
    spent: &mut usize,
) -> Result<usize, Diagnostic> {
    let children = node
        .fields()
        .len()
        .checked_add(node.variants().len())
        .and_then(|n| n.checked_add(usize::from(node.element().is_some())))
        .ok_or_else(budget)?;
    work::take(spent, children)?;
    work::retained::<VerifiedCloneNode<'p, 'a>>(spent, children)?;
    #[cfg(test)]
    work::reserve_seen(children);
    stack.try_reserve(children).map_err(|_| budget())?;
    for (_, child) in node.fields() {
        stack.push(child);
    }
    for (_, child) in node.variants() {
        if let Some(child) = child {
            stack.push(child);
        }
    }
    if let Some(child) = node.element() {
        stack.push(child);
    }
    Ok(children)
}
