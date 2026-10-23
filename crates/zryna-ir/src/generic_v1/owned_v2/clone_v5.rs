//! Operation-bound Struct Clone recipes retained only by the independent v5 issuer.
use super::{Failure, graph, plan, raw};
use crate::generic_v1::{budget, reject, reserve};
use zryna_layout::generic_v1::{TypeView, VerifiedLayouts};
mod recipe;
mod sites;
mod views;
pub use views::{CloneNodeKind, VerifiedCloneNode, VerifiedStructClone};

#[derive(Debug)]
struct Edge {
    ordinal: u32,
    start: u64,
    child: Option<u32>,
}
#[derive(Debug)]
struct Node {
    ty: u32,
    kind: CloneNodeKind,
    strings: u64,
    work: u64,
    edges: Vec<Edge>,
}
#[derive(Debug)]
struct Site {
    function: usize,
    result: u32,
    source: u32,
    ty: u32,
    root: u32,
    block: u32,
    position: u32,
    failure: Option<usize>,
}
#[derive(Default)]
pub(super) struct Table {
    nodes: Vec<Node>,
    sites: Vec<Site>,
}
impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructCloneTable")
            .field("nodes", &self.nodes.len())
            .field("sites", &self.sites.len())
            .finish()
    }
}
impl Table {
    pub(super) fn site(
        &self,
        function: usize,
        result: u32,
        used: &mut usize,
    ) -> Result<Option<(bool, u32)>, Failure> {
        let mut first = 0;
        let mut end = self.sites.len();
        while first < end {
            let middle = first + (end - first) / 2;
            plan::charge(used, 1)?;
            let site = &self.sites[middle];
            match (site.function, site.result).cmp(&(function, result)) {
                std::cmp::Ordering::Less => first = middle + 1,
                std::cmp::Ordering::Greater => end = middle,
                std::cmp::Ordering::Equal => {
                    return Ok(Some((self.nodes[site.root as usize].strings != 0, site.source)));
                }
            }
        }
        Ok(None)
    }
    fn lookup(&self, function: usize, result: u32) -> Option<&Site> {
        self.sites
            .binary_search_by_key(&(function, result), |s| (s.function, s.result))
            .ok()
            .and_then(|i| self.sites.get(i))
    }
    pub(super) fn views<'p>(
        &'p self,
        plans: &'p [raw::Plan],
    ) -> impl Iterator<Item = VerifiedStructClone<'p>> {
        self.sites.iter().map(move |site| VerifiedStructClone {
            table: self,
            site,
            failure: site.failure.map(|index| &plans[site.function].steps[index]),
        })
    }
    pub(super) fn view<'p>(
        &'p self,
        function: usize,
        result: u32,
        plans: &'p [raw::Plan],
    ) -> Option<VerifiedStructClone<'p>> {
        let site = self.lookup(function, result)?;
        let failure = site.failure.and_then(|index| plans.get(function)?.steps.get(index));
        if (self.nodes.get(site.root as usize)?.strings != 0) != failure.is_some() {
            return None;
        }
        Some(VerifiedStructClone { table: self, site, failure })
    }
}
fn add(a: u64, b: u64) -> Result<u64, Failure> {
    a.checked_add(b).ok_or_else(|| budget("Struct clone occurrence count overflow"))
}
