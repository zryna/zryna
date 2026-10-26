//! Prepay both complete traversals before retaining native clone projections.
use super::{CloneKind, CloneNode, CloneSite, VerifiedOwnedProgram, add, budget, invariant, width};
use crate::generic_owned_v2::{MAX_TYPE_LANES, plan::take};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::clone_v5::{CloneNodeKind, VerifiedCloneNode};

pub(in crate::generic_owned_v2) fn preflight(
    program: &VerifiedOwnedProgram<'_>,
    units: &mut u64,
) -> Result<(), Diagnostic> {
    for proof in program.struct_clones() {
        // Reject compact-DAG String amplification before recursive fanout,
        // native scratch allocation, cursor literals or code generation.
        take(units, proof.string_occurrences().checked_mul(128).ok_or_else(budget)?)?;
        // Site lookup/sorting and complete stack initialization/result loads.
        take(units, 256 + u64::from(MAX_TYPE_LANES) * 16)?;
        charge(proof.root(), units, 0)?;
    }
    Ok(())
}

fn charge(node: VerifiedCloneNode<'_>, units: &mut u64, depth: usize) -> Result<(), Diagnostic> {
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget());
    }
    // Covers complete projection, forward emission and one reverse guarded cleanup.
    take(units, 256)?;
    for (_, _, child) in node.fields() {
        charge(child, units, depth + 1)?;
    }
    for (_, _, child) in node.variants() {
        take(units, 256)?;
        if let Some(child) = child {
            charge(child, units, depth + 1)?;
        }
    }
    Ok(())
}

pub(in crate::generic_owned_v2) fn derive<'p>(
    program: &'p VerifiedOwnedProgram<'_>,
    widths: &[Option<u32>],
) -> Result<Vec<CloneSite<'p>>, Diagnostic> {
    let mut sites = Vec::new();
    for proof in program.struct_clones() {
        let root = project(proof.root(), widths, 0, 0, 0)?;
        if root.width != width(widths, proof.ty())? {
            return Err(invariant());
        }
        sites.try_reserve(1).map_err(|_| budget())?;
        sites.push(CloneSite { function: proof.function(), proof, root });
    }
    sites.sort_unstable_by_key(CloneSite::key);
    Ok(sites)
}

fn project(
    node: VerifiedCloneNode<'_>,
    widths: &[Option<u32>],
    begin: u32,
    string_start: u64,
    depth: usize,
) -> Result<CloneNode, Diagnostic> {
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget());
    }
    let width = width(widths, node.ty())?;
    let kind = match node.kind() {
        CloneNodeKind::Copy => CloneKind::Copy,
        CloneNodeKind::String => CloneKind::String,
        CloneNodeKind::Struct => CloneKind::Struct,
        CloneNodeKind::Enum => CloneKind::Enum,
    };
    let mut children = Vec::new();
    if kind == CloneKind::Struct {
        children.try_reserve(node.fields().len()).map_err(|_| budget())?;
        let mut lane = begin;
        for (ordinal, relative, child) in node.fields() {
            if ordinal as usize != children.len() {
                return Err(invariant());
            }
            let child_start = string_start.checked_add(relative).ok_or_else(budget)?;
            let child = project(child, widths, lane, child_start, depth + 1)?;
            lane = add(lane, child.width)?;
            children.push((ordinal, Some(child)));
        }
        if lane != add(begin, width)? {
            return Err(invariant());
        }
    } else if kind == CloneKind::Enum {
        children.try_reserve(node.variants().len()).map_err(|_| budget())?;
        for (ordinal, relative, child) in node.variants() {
            if ordinal as usize != children.len() {
                return Err(invariant());
            }
            let child_start = string_start.checked_add(relative).ok_or_else(budget)?;
            let child = child
                .map(|child| project(child, widths, add(begin, 1)?, child_start, depth + 1))
                .transpose()?;
            if child.as_ref().is_some_and(|child| child.width >= width) {
                return Err(invariant());
            }
            children.push((ordinal, child));
        }
    } else if kind == CloneKind::String && (width != 3 || node.string_occurrences() != 1) {
        return Err(invariant());
    }
    if add(begin, width)? > MAX_TYPE_LANES {
        return Err(budget());
    }
    Ok(CloneNode { kind, begin, width, string_start, children })
}
