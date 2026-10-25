//! Independently recover actual operation-reachable Clone permissions, never the emitter cache.
use super::{CloneNodeKind, Diagnostic, Model, VerifiedCloneNode, budget, clone_kind, invariant};
pub(super) fn derive(m: &Model<'_, '_, '_>, n: usize) -> Result<Vec<u32>, Diagnostic> {
    m.take(
        n.checked_mul(size_of::<u32>() + size_of::<VerifiedCloneNode<'_, '_>>() + 8)
            .ok_or_else(budget)?,
    )?;
    let mut kinds = Vec::new();
    kinds.try_reserve_exact(n).map_err(|_| budget())?;
    kinds.resize(n, 0);
    let mut stack = Vec::new();
    stack.try_reserve_exact(n).map_err(|_| budget())?;
    m.take(m.program.operations().len())?;
    for operation in m.program.operations() {
        if let Some(frontier) = operation.clone_frontier() {
            push(&mut kinds, &mut stack, frontier.frontier())?;
            while let Some(node) = stack.pop() {
                match node.kind() {
                    CloneNodeKind::Enum => {
                        m.take(node.variants().len())?;
                        for (_, child) in node.variants() {
                            if let Some(child) = child {
                                push(&mut kinds, &mut stack, child)?;
                            }
                        }
                    }
                    CloneNodeKind::Struct => {
                        m.take(node.fields().len())?;
                        for (_, child) in node.fields() {
                            push(&mut kinds, &mut stack, child)?;
                        }
                    }
                    CloneNodeKind::FixedArray { .. } | CloneNodeKind::VecEach => {
                        push(&mut kinds, &mut stack, node.element().ok_or_else(invariant)?)?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(kinds)
}
fn push<'p, 'a>(
    kinds: &mut [u32],
    stack: &mut Vec<VerifiedCloneNode<'p, 'a>>,
    node: VerifiedCloneNode<'p, 'a>,
) -> Result<(), Diagnostic> {
    let kind = kinds.get_mut(node.ty() as usize).ok_or_else(invariant)?;
    let expected = clone_kind(node.kind());
    if *kind != 0 {
        if *kind != expected {
            return Err(invariant());
        }
        return Ok(());
    }
    if stack.len() == stack.capacity() {
        return Err(invariant());
    }
    *kind = expected;
    stack.push(node);
    Ok(())
}
