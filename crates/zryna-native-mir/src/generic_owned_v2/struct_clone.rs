//! Operation-bound clone authority with checked flattened lanes, separate from memory-record MIR.
use super::{budget, invariant};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::owned_v2::{self, VerifiedOwnedProgram};

mod planning;
pub(super) use planning::{derive, preflight};

/// The independently proved role of a private clone region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloneKind {
    /// Complete Copy region, with no String ownership.
    Copy,
    /// Exactly one three-lane String owner.
    String,
    /// Declaration-ordered fields.
    Struct,
    /// Disjoint, ordinal-indexed possible payloads.
    Enum,
}

/// Immutable lane projection derived only from an IR operation-bound clone proof.
///
/// ```compile_fail
/// use zryna_native_mir::generic_owned_v2::{CloneKind, CloneNode};
/// let _ = CloneNode {
///     kind: CloneKind::String, begin: 0, width: 3, string_start: 0, children: Vec::new(),
/// };
/// ```
#[derive(Debug)]
pub struct CloneNode {
    kind: CloneKind,
    begin: u32,
    width: u32,
    string_start: u64,
    children: Vec<(u32, Option<CloneNode>)>,
}

impl CloneNode {
    /// Exact independently proved region role.
    #[must_use]
    pub const fn kind(&self) -> CloneKind {
        self.kind
    }
    /// Checked absolute flattened destination/source lane start.
    #[must_use]
    pub const fn begin(&self) -> u32 {
        self.begin
    }
    /// Checked complete lane width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Absolute static String occurrence, including inactive enum alternatives.
    #[must_use]
    pub const fn string_start(&self) -> u64 {
        self.string_start
    }
    /// Declaration-order fields or ordinal-order enum alternatives.
    #[must_use]
    pub fn children(
        &self,
    ) -> impl DoubleEndedIterator<Item = (u32, Option<&Self>)> + ExactSizeIterator {
        self.children.iter().map(|(ordinal, child)| (*ordinal, child.as_ref()))
    }
}

#[derive(Debug)]
pub(super) struct CloneSite<'p> {
    function: usize,
    proof: owned_v2::clone_v5::VerifiedStructClone<'p>,
    root: CloneNode,
}

impl CloneSite<'_> {
    pub(super) fn key(&self) -> (usize, u32) {
        (self.function, self.proof.result())
    }
    pub(super) fn view(&self) -> VerifiedStructClone<'_> {
        VerifiedStructClone {
            source: self.proof.source(),
            root: &self.root,
            step: self.proof.failure_step(),
        }
    }
}

/// Read-only native clone site retaining its exact independently verified IR authority.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedStructClone<'p> {
    source: u32,
    root: &'p CloneNode,
    step: Option<&'p owned_v2::raw::Step>,
}

impl<'p> VerifiedStructClone<'p> {
    /// Complete live source; clone never consumes its lanes.
    #[must_use]
    pub const fn source(self) -> u32 {
        self.source
    }
    /// Complete checked clone/range tree.
    #[must_use]
    pub const fn root(self) -> &'p CloneNode {
        self.root
    }
    /// Pre-result failure snapshot; local prefix release must precede this Step.
    #[must_use]
    pub const fn failure_step(self) -> Option<&'p owned_v2::raw::Step> {
        self.step
    }
}

fn width(widths: &[Option<u32>], ty: u32) -> Result<u32, Diagnostic> {
    widths.get(ty as usize).copied().flatten().ok_or_else(invariant)
}

fn add(a: u32, b: u32) -> Result<u32, Diagnostic> {
    a.checked_add(b).ok_or_else(budget)
}
