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
    /// Allocator begins exactly after all static intervals, aligned to eight.
    pub heap_start: u32,
    /// Complete new-domain function memory inventory.
    pub functions: Vec<FunctionFrame>,
}
