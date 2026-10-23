//! Canonical same-physical static paths and source-complete immediate decompositions.
use super::{Failure, Kind, Row, TypeWorld, budget, raw, reject, reserve, resource_failure};
#[derive(Clone, Copy, Debug)]
enum Required {
    Terminal,
    All(usize, bool),
    Active,
}
#[derive(Debug)]
struct Node {
    canonical: u32,
    parent: Option<u32>,
    first: Option<u32>,
    next: Option<u32>,
    count: usize,
    ordinal: u32,
    required: Required,
    begin: usize,
    end: usize,
}
#[derive(Debug, Default)]
pub(in crate::generic_v1::execution_v1) struct Decomposition {
    nodes: Vec<Node>,
}
impl Decomposition {
    pub(in crate::generic_v1::execution_v1) fn build(
        f: &raw::Function,
        rows: &[Row],
        world: &TypeWorld,
        work: &mut usize,
    ) -> Result<Self, Failure> {
        budget::charge(
            work,
            rows.len()
                .checked_mul(16)
                .ok_or_else(|| resource_failure("static decomposition table overflow"))?,
        )?;
        let mut nodes: Vec<Node> = reserve(rows.len())?;
        for (index, row) in rows.iter().enumerate() {
            let id = u32::try_from(index).map_err(|_| Failure::InternalFailure)?;
            let parent = row.parent.map(|p| nodes[p as usize].canonical);
            let ordinal = match f.places[index].kind {
                raw::PlaceKind::StructField { ordinal, .. } => ordinal,
                raw::PlaceKind::EnumPayload { variant, .. } => variant,
                raw::PlaceKind::FixedArrayConstant { index, .. } => index,
                _ => 0,
            };
            let required = match world
                .kind(row.ty)
                .ok_or_else(|| reject("decomposition source type absent"))?
            {
                Kind::Struct { .. } => Required::All(
                    world.members(row.ty).ok_or_else(|| reject("Struct members absent"))?.len(),
                    false,
                ),
                Kind::FixedArray { length } => Required::All(length as usize, true),
                Kind::Enum { .. } | Kind::Option | Kind::Result => Required::Active,
                _ => Required::Terminal,
            };
            let (canonical, previous, next) = if let Some(parent) = parent {
                budget::charge(
                    work,
                    nodes[parent as usize]
                        .count
                        .checked_add(1)
                        .ok_or_else(|| resource_failure("canonical sibling walk overflow"))?,
                )?;
                let mut previous = None;
                let mut next = nodes[parent as usize].first;
                while let Some(at) = next {
                    if nodes[at as usize].ordinal >= ordinal {
                        break;
                    }
                    previous = Some(at);
                    next = nodes[at as usize].next;
                }
                let canonical =
                    next.filter(|at| nodes[*at as usize].ordinal == ordinal).unwrap_or(id);
                (canonical, previous, next)
            } else {
                (id, None, None)
            };
            nodes.push(Node {
                canonical,
                parent,
                first: None,
                next: None,
                count: 0,
                ordinal,
                required,
                begin: 0,
                end: 0,
            });
            if canonical == id
                && let Some(parent) = parent
            {
                nodes[index].next = next;
                if let Some(previous) = previous {
                    nodes[previous as usize].next = Some(id);
                } else {
                    nodes[parent as usize].first = Some(id);
                }
                nodes[parent as usize].count += 1;
            }
        }
        let mut result = Self { nodes };
        result.intervals(work)?;
        Ok(result)
    }
    fn intervals(&mut self, work: &mut usize) -> Result<(), Failure> {
        budget::charge(
            work,
            self.nodes
                .len()
                .checked_mul(4)
                .ok_or_else(|| resource_failure("static path interval work overflow"))?,
        )?;
        let mut clock = 0;
        for root in 0..self.nodes.len() {
            if self.nodes[root].canonical as usize != root || self.nodes[root].parent.is_some() {
                continue;
            }
            let mut at = root;
            'tree: loop {
                self.nodes[at].begin = clock;
                clock += 1;
                if let Some(child) = self.nodes[at].first {
                    at = child as usize;
                    continue;
                }
                loop {
                    self.nodes[at].end = clock;
                    if at == root {
                        break 'tree;
                    }
                    if let Some(next) = self.nodes[at].next {
                        at = next as usize;
                        break;
                    }
                    at = self.nodes[at].parent.expect("checked nonroot parent") as usize;
                }
            }
        }
        Ok(())
    }
    pub(in crate::generic_v1::execution_v1) fn canonical(&self, id: u32) -> u32 {
        self.nodes[id as usize].canonical
    }
    pub(in crate::generic_v1::execution_v1) fn parent(&self, id: u32) -> Option<u32> {
        self.nodes[self.canonical(id) as usize].parent
    }
    pub(in crate::generic_v1::execution_v1) fn contains(&self, parent: u32, child: u32) -> bool {
        let a = &self.nodes[self.canonical(parent) as usize];
        let b = &self.nodes[self.canonical(child) as usize];
        a.begin <= b.begin && b.begin < a.end
    }
    pub(in crate::generic_v1::execution_v1) fn child(
        &self,
        id: u32,
        projection: raw::StaticProjection,
    ) -> Option<u32> {
        let node = &self.nodes[self.canonical(id) as usize];
        let ((Required::All(_, false), raw::StaticProjection::StructField(ordinal))
        | (Required::All(_, true), raw::StaticProjection::FixedArrayConstant(ordinal))
        | (Required::Active, raw::StaticProjection::EnumPayload(ordinal))) =
            (node.required, projection)
        else {
            return None;
        };
        self.all(id).find(|child| self.nodes[*child as usize].ordinal == ordinal)
    }
    pub(in crate::generic_v1::execution_v1) fn all(&self, id: u32) -> Children<'_> {
        let n = &self.nodes[self.canonical(id) as usize];
        Children { graph: self, next: n.first, remaining: n.count }
    }
    pub(in crate::generic_v1::execution_v1) fn complete(
        &self,
        id: u32,
        active: Option<u32>,
    ) -> Result<Children<'_>, Failure> {
        let n = &self.nodes[self.canonical(id) as usize];
        match n.required {
            Required::All(count, _) if n.count == count => Ok(self.all(id)),
            Required::Active => {
                let active =
                    active.ok_or_else(|| reject("partial enum lacks active payload refinement"))?;
                let child = self
                    .all(id)
                    .find(|child| self.nodes[*child as usize].ordinal == active)
                    .ok_or_else(|| reject("active payload decomposition is missing"))?;
                Ok(Children { graph: self, next: Some(child), remaining: 1 })
            }
            _ => Err(reject("partial place lacks its complete original child decomposition")),
        }
    }
}
pub(in crate::generic_v1::execution_v1) struct Children<'a> {
    graph: &'a Decomposition,
    next: Option<u32>,
    remaining: usize,
}
impl Iterator for Children<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.remaining == 0 {
            return None;
        }
        let id = self.next?;
        self.next = self.graph.nodes[id as usize].next;
        self.remaining -= 1;
        Some(id)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl ExactSizeIterator for Children<'_> {}
