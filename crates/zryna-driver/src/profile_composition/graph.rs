use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sha2::{Digest, Sha256};
use zryna_diagnostics::Diagnostic;

use super::{
    INVALID,
    authority::Binding,
    error,
    model::{Input, VERSION},
};

pub(super) struct Graph {
    pub input: Input,
    pub children: BTreeMap<String, Vec<String>>,
}

impl Graph {
    pub(super) fn binding(&self, authorities: &Binding) -> Result<[u8; 32], Vec<Diagnostic>> {
        let mut bytes = serde_json::to_vec(&self.input)
            .map_err(|_| vec![error(INVALID, "cannot bind composition input")])?;
        bytes.extend_from_slice(format!("{authorities:?}").as_bytes());
        Ok(Sha256::digest(bytes).into())
    }

    pub(super) fn ids(&self) -> BTreeSet<String> {
        self.input.instances.iter().map(|node| node.id.clone()).collect()
    }

    // Sorted breadth-first discovery fixes shortest paths, then bytewise sequence ties.
    pub(super) fn paths(&self, root: &str) -> BTreeMap<String, Vec<String>> {
        let mut paths = BTreeMap::from([(root.to_owned(), vec![root.to_owned()])]);
        let mut pending = VecDeque::from([root.to_owned()]);
        while let Some(id) = pending.pop_front() {
            for child in &self.children[&id] {
                if !paths.contains_key(child) {
                    let mut path = paths[&id].clone();
                    path.push(child.clone());
                    paths.insert(child.clone(), path);
                    pending.push_back(child.clone());
                }
            }
        }
        paths
    }
}

fn invalid(message: &str) -> Vec<Diagnostic> {
    vec![error(INVALID, message)]
}

fn identity(value: &str, bytes: &mut usize) -> Result<(), Vec<Diagnostic>> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(invalid("empty or control-bearing composition identity"));
    }
    *bytes = bytes
        .checked_add(value.len())
        .filter(|total| *total <= 65_536)
        .ok_or_else(|| invalid("identity bytes limit 65536 exceeded at 65537"))?;
    Ok(())
}

// Count every identity occurrence, including references, before copying or traversing.
fn bounds(input: &Input) -> Result<(), Vec<Diagnostic>> {
    if input.version != VERSION || input.instances.is_empty() || input.instances.len() > 256 {
        return Err(invalid("version or instances bound (1..=256) is invalid"));
    }
    if input.edges.len() > 4096 || input.selections.is_empty() || input.selections.len() > 3 {
        return Err(invalid("edges limit 4096 or selected outputs bound (1..=3) exceeded"));
    }
    let mut bytes = 0;
    identity(&input.root, &mut bytes)?;
    identity(&input.version, &mut bytes)?;
    for selection in &input.selections {
        identity(&selection.policy_version, &mut bytes)?;
        if let Some(world) = &selection.world {
            identity(world, &mut bytes)?;
        }
        if selection.approved.len() > 13 {
            return Err(invalid("approved interface bound exceeded"));
        }
        for requirement in &selection.approved {
            identity(&requirement.interface, &mut bytes)?;
        }
    }
    for node in &input.instances {
        identity(&node.id, &mut bytes)?;
        if node.rows.is_empty() || node.requirements.len() > 13 {
            return Err(invalid("missing target compatibility or excessive direct requirements"));
        }
        for requirement in &node.requirements {
            identity(&requirement.interface, &mut bytes)?;
        }
        super::quota::validate_reservation(&node.reservation)?;
    }
    for (from, to) in &input.edges {
        identity(from, &mut bytes)?;
        identity(to, &mut bytes)?;
    }
    Ok(())
}

pub(super) fn validate(input: &Input) -> Result<Graph, Vec<Diagnostic>> {
    bounds(input)?;
    let mut input = input.clone();
    input.instances.sort_by(|a, b| a.id.cmp(&b.id));
    input.edges.sort();
    input.selections.sort_by_key(|selection| selection.row);
    let ids: BTreeSet<_> = input.instances.iter().map(|node| node.id.clone()).collect();
    if ids.len() != input.instances.len() || !ids.contains(&input.root) {
        return Err(invalid("duplicate instance identity or missing root"));
    }
    if input.edges.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("duplicate dependency edge"));
    }
    let mut children: BTreeMap<_, Vec<_>> = ids.iter().map(|id| (id.clone(), Vec::new())).collect();
    let mut incoming: BTreeMap<_, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    for (from, to) in &input.edges {
        if !ids.contains(from) || !ids.contains(to) {
            return Err(invalid("dangling dependency edge"));
        }
        children.get_mut(from).expect("validated source").push(to.clone());
        *incoming.get_mut(to).expect("validated destination") += 1;
    }
    let graph = Graph { input, children };
    if graph.paths(&graph.input.root).len() != ids.len() {
        return Err(invalid("graph contains unreachable instances"));
    }
    let mut ready: BTreeSet<_> =
        incoming.iter().filter(|(_, n)| **n == 0).map(|(id, _)| id.clone()).collect();
    let mut depths = BTreeMap::from([(graph.input.root.clone(), 0)]);
    let mut visited = 0;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        let depth = depths[&id];
        for child in &graph.children[&id] {
            let next = depths.entry(child.clone()).or_default();
            *next = (*next).max(depth + 1);
            if *next > 32 {
                return Err(invalid("dependency depth limit 32 exceeded at 33"));
            }
            let remaining = incoming.get_mut(child).expect("validated child");
            *remaining -= 1;
            if *remaining == 0 {
                ready.insert(child.clone());
            }
        }
    }
    if visited != ids.len() {
        return Err(invalid("dependency cycle"));
    }
    Ok(graph)
}
