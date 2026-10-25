//! Separate graph relations; actual row, edge, queue and retained-slot work is prepaid.
use super::{Fact, Failure, Kind, Node, add, charge, exhausted, reject, reserve};
pub(super) fn derive(nodes: &[Node], spent: &mut usize) -> Result<Vec<Fact>, Failure> {
    charge(spent, nodes.len())?;
    let mut edges = 0;
    for node in nodes {
        if !node.ready {
            return Err(reject("TypeWorld has an incomplete nominal definition"));
        }
        edges = add(edges, add(node.arguments.len(), node.members.len())?)?;
    }
    if edges > zryna_layout::MAX_DEPENDENCY_EDGES {
        return Err(exhausted("TypeWorld dependency edge ceiling"));
    }
    for node in nodes {
        row(node, spent)?;
        if node
            .arguments
            .iter()
            .chain(node.members.iter().flatten())
            .any(|id| *id as usize >= nodes.len())
        {
            return Err(reject("TypeWorld edge names an absent type"));
        }
    }
    let arguments = reverse(nodes, Relation::Arguments, spent)?;
    acyclic(nodes, &arguments, Relation::Arguments, spent)?;
    let storage = reverse(nodes, Relation::Storage, spent)?;
    acyclic(nodes, &storage, Relation::Storage, spent)?;
    let parents = reverse(nodes, Relation::Capability, spent)?;
    let copy = fixed_point(nodes, &parents, false, spent)?;
    let clone = fixed_point(nodes, &parents, true, spent)?;
    charge(spent, nodes.len())?;
    let mut facts = reserve(nodes.len())?;
    facts.extend(copy.into_iter().zip(clone).map(|(copy, clone)| Fact { copy, clone }));
    Ok(facts)
}
#[derive(Clone, Copy)]
enum Relation {
    Arguments,
    Storage,
    Capability,
}
fn row(node: &Node, spent: &mut usize) -> Result<(), Failure> {
    charge(spent, add(1, add(node.arguments.len(), node.members.len())?)?)
}
fn dependencies(node: &Node, relation: Relation) -> impl Iterator<Item = u32> + '_ {
    let members = !matches!(relation, Relation::Arguments)
        && matches!(
            node.kind,
            Kind::Struct { .. } | Kind::Enum { .. } | Kind::Option | Kind::Result
        );
    let arguments = matches!(relation, Relation::Arguments)
        || matches!(node.kind, Kind::FixedArray { .. })
        || matches!(relation, Relation::Capability) && node.kind == Kind::Vec;
    node.members
        .iter()
        .filter_map(move |id| if members { *id } else { None })
        .chain(node.arguments.iter().copied().filter(move |_| arguments))
}
fn reverse(
    nodes: &[Node],
    relation: Relation,
    spent: &mut usize,
) -> Result<Vec<Vec<u32>>, Failure> {
    charge(spent, nodes.len())?;
    let mut counts = reserve(nodes.len())?;
    counts.resize(nodes.len(), 0usize);
    for node in nodes {
        row(node, spent)?;
        for child in dependencies(node, relation) {
            charge(spent, 1)?;
            counts[child as usize] = add(counts[child as usize], 1)?;
        }
    }
    charge(spent, nodes.len())?;
    let mut parents = reserve(nodes.len())?;
    for count in counts {
        charge(spent, count)?;
        parents.push(reserve(count)?);
    }
    for (id, node) in nodes.iter().enumerate() {
        row(node, spent)?;
        let id = u32::try_from(id).map_err(|_| Failure::InternalFailure)?;
        for child in dependencies(node, relation) {
            charge(spent, 1)?;
            parents[child as usize].push(id);
        }
    }
    Ok(parents)
}
fn acyclic(
    nodes: &[Node],
    parents: &[Vec<u32>],
    relation: Relation,
    spent: &mut usize,
) -> Result<(), Failure> {
    charge(spent, add(nodes.len(), nodes.len())?)?;
    let mut remaining = reserve(nodes.len())?;
    let mut ready = reserve(nodes.len())?;
    for (id, node) in nodes.iter().enumerate() {
        row(node, spent)?;
        let count = dependencies(node, relation).count();
        remaining.push(count);
        if count == 0 {
            ready.push(id);
        }
    }
    let mut at = 0;
    while at < ready.len() {
        charge(spent, 1)?;
        let id = ready[at];
        at += 1;
        charge(spent, parents[id].len())?;
        for parent in &parents[id] {
            let degree = &mut remaining[*parent as usize];
            *degree = degree.checked_sub(1).ok_or(Failure::InternalFailure)?;
            if *degree == 0 {
                ready.push(*parent as usize);
            }
        }
    }
    if ready.len() != nodes.len() {
        return Err(reject(
            "TypeWorld contains a cyclic supplied argument or by-value storage graph",
        ));
    }
    Ok(())
}
fn fixed_point(
    nodes: &[Node],
    parents: &[Vec<u32>],
    cloning: bool,
    spent: &mut usize,
) -> Result<Vec<bool>, Failure> {
    charge(spent, add(nodes.len(), nodes.len())?)?;
    let mut facts = reserve(nodes.len())?;
    let mut pending = reserve(nodes.len())?;
    for (id, node) in nodes.iter().enumerate() {
        charge(spent, 1)?;
        let available = match node.kind {
            Kind::Opaque { .. } => false,
            Kind::String | Kind::Vec | Kind::Shared | Kind::Weak => cloning,
            _ => true,
        };
        facts.push(available);
        if !available {
            pending.push(id);
        }
    }
    let mut at = 0;
    while at < pending.len() {
        charge(spent, 1)?;
        let id = pending[at];
        at += 1;
        charge(spent, parents[id].len())?;
        for parent in &parents[id] {
            if facts[*parent as usize] {
                facts[*parent as usize] = false;
                pending.push(*parent as usize);
            }
        }
    }
    Ok(facts)
}
