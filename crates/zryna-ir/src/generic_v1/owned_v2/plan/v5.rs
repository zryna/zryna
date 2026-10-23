//! V5 recipes and ordinary owner replay spend one inherited ledger.
use super::super::clone_v5::Table;
use super::{Failure, State, VerifiedLayouts, graph, raw};
pub(in super::super) fn derive_counted(
    program: &graph::Program,
    extensions: &[Vec<raw::Extension>],
    layouts: &VerifiedLayouts,
) -> Result<(Vec<raw::Plan>, usize, Table), Failure> {
    let mut used = 0;
    let mut clones = Table::derive(program, extensions, layouts, &mut used)?;
    let (plans, mut used) = super::replay_with(program, extensions, layouts, Some(&clones), used)?;
    clones.validate_plans(&plans, &mut used)?;
    Ok((plans, used, clones))
}
pub(super) fn clone(
    plan: &mut raw::Plan,
    state: &State,
    location: (usize, usize, usize, u32),
    operand: (u32, graph::Type, bool),
    clones: Option<&Table>,
    used: &mut usize,
) -> Result<(), Failure> {
    let (function, block, position, result) = location;
    let (source, ty, owned) = operand;
    let table =
        clones.ok_or_else(|| super::reject("Struct Clone is outside private V5 plan admission"))?;
    let (fallible, expected) = table
        .site(function, result, used)?
        .ok_or_else(|| super::reject("Struct Clone has no exact operation-bound recipe"))?;
    if source != expected {
        return Err(super::reject("Struct Clone source differs from its operation-bound recipe"));
    }
    state.read(source, ty, owned)?;
    if fallible {
        super::step(plan, state, block, position, true, used)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
