//! Untrusted target memory claims, distinct from the opaque verified plan.
use zryna_ir::generic_v1::execution_v1::{OperationSite, VerifiedExecution, raw::ValueType};
/// One compiler-owned stored value or place interval relative to its invocation frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slot {
    /// Exact same-domain stored type or unit.
    pub ty: ValueType,
    /// Frame-relative byte offset.
    pub offset: u32,
    /// Exact stored byte size.
    pub bytes: u32,
    /// Exact stored alignment.
    pub alignment: u32,
}
/// One exact operation's unpublished Vec descriptor; no source owner or SSA ID is invented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingVec {
    /// Exact original append operation.
    pub site: OperationSite,
    /// Same-Vec typed private pending descriptor interval.
    pub slot: Slot,
}
/// Pure compiler snapshots of one actual upgrade successor's ordinary arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEdge {
    /// Original upgrade block.
    pub block: u32,
    /// Selects the success ordinary edge; no generated binding is included.
    pub success: bool,
    /// Exact ordered argument shapes in disjoint private frame storage.
    pub arguments: Vec<Slot>,
}
/// Private address cell for one actual nonstored loan; never a referent Place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoanCell {
    /// Exact stored referent identity in this same execution/layout universe.
    pub referent: u32,
    /// Exact checked access, not a new permission grant.
    pub access: zryna_ir::generic_v1::execution_v1::raw::Access,
    /// Four-aligned frame offset of one absolute linear address.
    pub offset: u32,
}
/// Complete claimed frame and acyclic source-call peak.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionFrame {
    /// Eight-aligned invocation size, without heap resource allocation.
    pub bytes: u32,
    /// Invocation plus deepest simultaneously active source call chain.
    pub peak: u32,
    /// Dense SSA slots indexed by value identity.
    pub values: Vec<Slot>,
    /// Dense place offsets, including independently checked projection aliases.
    pub places: Vec<Slot>,
    /// Source-ordered exact new-domain callee inventory indices.
    pub calls: Vec<u32>,
    /// Dense actual loan identities, separate from ordinary values and places.
    pub loans: Vec<LoanCell>,
    /// Source-ordered unpublished append descriptors.
    pub pending_vecs: Vec<PendingVec>,
    /// Block-ordered snapshots, expiration before success for each upgrade.
    pub upgrade_edges: Vec<UpgradeEdge>,
}
/// Source-authenticated literal retained as actual static bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Literal {
    /// Exact same-program operation identity.
    pub site: OperationSite,
    /// Absolute static address.
    pub offset: u32,
    /// Exact original UTF8 bytes.
    pub bytes: Vec<u8>,
}
/// Untrusted compiler-private continuation storage, separate from source values and the heap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecursiveArena {
    /// Eight-aligned immutable interpreter metadata start following source-call frames.
    pub metadata_start: u32,
    /// Exact checked closed-type/member metadata reservation, aligned to eight.
    pub metadata_bytes: u32,
    /// Eight-aligned absolute continuation start immediately following interpreter metadata.
    pub start: u32,
    /// Exact capacity times the fixed Linear32 continuation-frame stride.
    pub bytes: u32,
    /// Storage capacity proved from the remaining physical allocator extent.
    pub capacity: u32,
}
/// A complete memory claim; construction alone grants no address authority.
#[derive(Clone, Debug)]
pub struct TargetMemoryPlan<'p, 'a> {
    /// Exact retained execution claim owner, not a substitutable stable fingerprint.
    pub program: &'p VerifiedExecution<'a>,
    /// Explicit separate test-instrumentation policy.
    pub observed: bool,
    /// Fixed inherited owned-memory page count.
    pub memory_pages: u32,
    /// Exclusive end of the reserved null interval beginning at zero.
    pub null_end: u32,
    /// Scalar wrapper result/helper scratch start.
    pub scratch_start: u32,
    /// Exact scratch interval size.
    pub scratch_bytes: u32,
    /// Test-only bounded trace interval start, or zero for ordinary output.
    pub trace_start: u32,
    /// Test-only trace interval size, or zero for ordinary output.
    pub trace_bytes: u32,
    /// Actual static literal inventory.
    pub literals: Vec<Literal>,
    /// Exact bounded compiler invocation arena start.
    pub frames_start: u32,
    /// Maximum acyclic source-call peak reserved as zero static data.
    pub frames_bytes: u32,
    /// Mandatory private arena for an actual recursive owned traversal graph, otherwise absent.
    /// This storage claim grants no Clone, Drop, source-address or canonical-header permission.
    pub recursive: Option<RecursiveArena>,
    /// Allocator begins exactly after all static intervals, aligned to eight.
    pub heap_start: u32,
    /// Complete new-domain function memory inventory.
    pub functions: Vec<FunctionFrame>,
}
