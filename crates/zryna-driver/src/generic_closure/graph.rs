//! Iterative final graph validation and a domain distinct from every older profile.
use super::{
    BTreeMap, BTreeSet, ModuleClosureError, ModuleEdge, ModuleRecord, NormalizedSourcePath, support,
};
use support::{add, diagnostic, invariant, rejected};
pub(super) fn validate(
    modules: &[ModuleRecord],
    edges: &[ModuleEdge],
) -> Result<(), ModuleClosureError> {
    let mut outgoing =
        modules.iter().map(|row| (row.path.clone(), BTreeSet::new())).collect::<BTreeMap<_, _>>();
    let mut incoming =
        modules.iter().map(|row| (row.path.clone(), 0usize)).collect::<BTreeMap<_, _>>();
    for edge in edges {
        if outgoing.get_mut(&edge.importer).ok_or_else(invariant)?.insert(edge.target.clone()) {
            let count = incoming.get_mut(&edge.target).ok_or_else(invariant)?;
            *count = add(*count, 1)?;
        }
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(path, count)| (*count == 0).then_some(path.clone()))
        .collect::<BTreeSet<_>>();
    let mut visited = 0;
    while let Some(path) = ready.pop_first() {
        visited = add(visited, 1)?;
        for target in outgoing.get(&path).ok_or_else(invariant)? {
            let count = incoming.get_mut(target).ok_or_else(invariant)?;
            *count = count.checked_sub(1).ok_or_else(invariant)?;
            if *count == 0 {
                ready.insert(target.clone());
            }
        }
    }
    if visited != modules.len() {
        return Err(rejected(diagnostic("ZRYNA-D3007", None, "generic module import cycle")));
    }
    Ok(())
}
pub(super) fn identity(
    entry: &NormalizedSourcePath,
    modules: &[ModuleRecord],
    edges: &[ModuleEdge],
) -> Result<[u8; 32], ModuleClosureError> {
    crate::source_graph_identity::hash(
        b"ZRYNA-BOUNDED-GENERICS-GRAPH-V1\0",
        entry,
        modules,
        edges,
        invariant,
    )
}
