//! Actual static memory ownership, independently checked before any Wasm emission.
use super::{Diagnostic, budget, invariant};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
mod peak;
pub mod raw;
pub(super) mod shape;
/// Existing M3 owned-memory ceiling; never the imported V5 memory size.
pub const MEMORY_PAGES: u32 = 256;
/// Maximum private linear address extent.
pub const MEMORY_BYTES: u32 = MEMORY_PAGES * 65536;
/// Opaque same-program/runtime/layout memory authority.
#[derive(Debug)]
pub struct VerifiedTargetMemoryPlan<'p, 'a> {
    pub(super) claim: raw::TargetMemoryPlan<'p, 'a>,
}
impl<'p, 'a> VerifiedTargetMemoryPlan<'p, 'a> {
    /// Immutable verified address claims; mutation requires independent verification again.
    #[must_use]
    pub const fn claims(&self) -> &raw::TargetMemoryPlan<'p, 'a> {
        &self.claim
    }
    /// Exact retained execution owner, not a byte-equivalent replacement.
    #[must_use]
    pub fn is_bound_to(&self, program: &VerifiedExecution<'_>) -> bool {
        std::ptr::eq(self.claim.program, program)
    }
}
/// Derives untrusted intervals using the new seal and its actual closed layouts.
/// # Errors
/// Rejects checked arithmetic, inherited target memory and retained-work limits.
pub fn derive<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    observed: bool,
) -> Result<raw::TargetMemoryPlan<'p, 'a>, Diagnostic> {
    let mut work = 0;
    let (mut functions, literals) = shape::inventory(program, &mut work)?;
    peak::fill(&mut functions, &mut work)?;
    let scratch_start = 8;
    let scratch_bytes = 32;
    let trace_start = if observed { 40 } else { 0 };
    let trace_bytes = if observed { 16384 } else { 0 };
    let mut cursor = 40u32.checked_add(trace_bytes).ok_or_else(budget)?;
    super::work::retained::<raw::Literal>(&mut work, literals.len())?;
    let mut static_literals = Vec::new();
    static_literals.try_reserve(literals.len()).map_err(|_| budget())?;
    for (site, bytes) in literals {
        let offset = cursor;
        cursor = add(cursor, u32::try_from(bytes.len()).map_err(|_| budget())?)?;
        static_literals.push(raw::Literal { site, offset, bytes });
    }
    let frames_start = align(cursor, 8)?;
    let frames_bytes = functions.iter().map(|f| f.peak).max().unwrap_or(0);
    let heap_start = align(add(frames_start, frames_bytes)?, 8)?;
    Ok(raw::TargetMemoryPlan {
        program,
        observed,
        memory_pages: MEMORY_PAGES,
        null_end: 8,
        scratch_start,
        scratch_bytes,
        trace_start,
        trace_bytes,
        literals: static_literals,
        frames_start,
        frames_bytes,
        heap_start,
        functions,
    })
}
/// Independently recomputes sizes, alignment, source-call peak and every interval.
/// # Errors
/// Rejects foreign owners, aliases, altered frames/literals/scratch/trace/heap and overflow.
pub fn verify<'p, 'a>(
    claim: raw::TargetMemoryPlan<'p, 'a>,
    program: &'p VerifiedExecution<'a>,
) -> Result<VerifiedTargetMemoryPlan<'p, 'a>, Diagnostic> {
    if !std::ptr::eq(claim.program, program)
        || !program.runtime().is_bound_to(
            program.layouts(zryna_layout::StorageTarget::Linear32V1),
            program.layouts(zryna_layout::StorageTarget::LinuxX8664V1),
        )
    {
        return Err(invariant());
    }
    let mut work = 0;
    shape::claim_census(&claim, &mut work)?;
    if claim.memory_pages != MEMORY_PAGES
        || claim.null_end != 8
        || claim.scratch_start != 8
        || claim.scratch_bytes != 32
        || claim.trace_bytes != if claim.observed { 16384 } else { 0 }
        || claim.trace_start != if claim.observed { 40 } else { 0 }
    {
        return Err(invariant());
    }
    let (mut expected, literals) = shape::inventory(program, &mut work)?;
    peak::fill(&mut expected, &mut work)?;
    if expected != claim.functions || literals.len() != claim.literals.len() {
        return Err(invariant());
    }
    let mut cursor = add(40, claim.trace_bytes)?;
    for ((site, bytes), actual) in literals.iter().zip(&claim.literals) {
        if actual.site != *site || actual.offset != cursor || actual.bytes != *bytes {
            return Err(invariant());
        }
        cursor = add(cursor, u32::try_from(bytes.len()).map_err(|_| budget())?)?;
    }
    let start = align(cursor, 8)?;
    let size = expected.iter().map(|f| f.peak).max().unwrap_or(0);
    if claim.frames_start != start
        || claim.frames_bytes != size
        || claim.heap_start != align(add(start, size)?, 8)?
    {
        return Err(invariant());
    }
    Ok(VerifiedTargetMemoryPlan { claim })
}
pub(super) fn add(a: u32, b: u32) -> Result<u32, Diagnostic> {
    a.checked_add(b).filter(|n| *n <= MEMORY_BYTES).ok_or_else(budget)
}
pub(super) fn align(value: u32, alignment: u32) -> Result<u32, Diagnostic> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(invariant());
    }
    add(value, (alignment - value % alignment) % alignment)
}
