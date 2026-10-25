//! Immutable recipe traversal; no caller can construct a view or a progress certificate.
use super::{Node, Site, Table, raw};

/// Closed recipe action, derived only after source and closed-layout verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloneNodeKind {
    /// Reuse a value with no owned String descendants.
    Copy,
    /// Clone one String resource.
    String,
    /// Visit declaration-ordered Struct fields.
    Struct,
    /// Visit the source-selected enum payload.
    Enum,
}

/// One exact same-seal instruction, destination and local failure frontier.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedStructClone<'p> {
    pub(super) table: &'p Table,
    pub(super) site: &'p Site,
    pub(super) failure: Option<&'p raw::Step>,
}
impl<'p> VerifiedStructClone<'p> {
    /// Owning function index in this sealed program.
    #[must_use]
    pub const fn function(self) -> usize {
        self.site.function
    }
    /// Complete retained source value ID.
    #[must_use]
    pub const fn source(self) -> u32 {
        self.site.source
    }
    /// Exact fresh result value ID.
    #[must_use]
    pub const fn result(self) -> u32 {
        self.site.result
    }
    /// Exact nominal Struct index in this program's retained layouts.
    #[must_use]
    pub const fn ty(self) -> u32 {
        self.site.ty
    }
    /// Complete immutable type-directed destination frontier.
    #[must_use]
    pub fn root(self) -> VerifiedCloneNode<'p> {
        VerifiedCloneNode { table: self.table, node: &self.table.nodes[self.site.root as usize] }
    }
    /// Static occurrence intervals include every possible enum alternative.
    #[must_use]
    pub fn string_occurrences(self) -> u64 {
        self.root().string_occurrences()
    }
    /// Outer cleanup runs after local initialized-prefix rollback; result is absent.
    #[must_use]
    pub const fn failure_step(self) -> Option<&'p raw::Step> {
        self.failure
    }
}
/// Opaque exact type node; repeated DAG children retain separate traversal intervals.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedCloneNode<'p> {
    table: &'p Table,
    node: &'p Node,
}
impl<'p> VerifiedCloneNode<'p> {
    /// Exact physical traversal action, independent of original source Clone authority.
    #[must_use]
    pub const fn kind(self) -> CloneNodeKind {
        self.node.kind
    }
    /// Type identity in the same retained layout domain.
    #[must_use]
    pub const fn ty(self) -> u32 {
        self.node.ty
    }
    /// Possible String occurrences beneath this node, counting all variant branches.
    #[must_use]
    pub const fn string_occurrences(self) -> u64 {
        self.node.strings
    }
    /// Declaration-ordered field ordinal, relative String interval start, and exact child.
    /// # Panics
    /// Panics only if the issuer's private Struct-child invariant is violated.
    #[must_use]
    pub fn fields(self) -> impl ExactSizeIterator<Item = (u32, u64, VerifiedCloneNode<'p>)> {
        let edges =
            if self.node.kind == CloneNodeKind::Struct { &self.node.edges[..] } else { &[] };
        edges.iter().map(move |e| {
            (
                e.ordinal,
                e.start,
                VerifiedCloneNode {
                    table: self.table,
                    node: &self.table.nodes[e.child.expect("sealed Struct child") as usize],
                },
            )
        })
    }
    /// Canonical variant ordinal, relative String interval start, and optional payload.
    #[must_use]
    pub fn variants(
        self,
    ) -> impl ExactSizeIterator<Item = (u32, u64, Option<VerifiedCloneNode<'p>>)> {
        let edges = if self.node.kind == CloneNodeKind::Enum { &self.node.edges[..] } else { &[] };
        edges.iter().map(move |e| {
            (
                e.ordinal,
                e.start,
                e.child.map(|id| VerifiedCloneNode {
                    table: self.table,
                    node: &self.table.nodes[id as usize],
                }),
            )
        })
    }
}
