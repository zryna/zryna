//! Same-program invocation plan and bounded preflight for NEW context/guard expansion.
use super::{Diagnostic, VerifiedProgram, budget, invariant, planning::take, shapes::Shape};
use zryna_ir::generic_v1::execution_v1::{
    VerifiedCleanupAction, VerifiedExecution, VerifiedOperation, raw,
};
use zryna_layout::TypeCategory;
use zryna_ownership_runtime_abi::LogicalOperation;
mod control;
mod prepay;
pub use control::ControlEffect;
mod vector;
pub use vector::VectorEffect;
/// Opaque same-seal invocation plan; raw names, pointers and counter models cannot construct it.
pub struct InvocationPlan<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    guarded_strings: u64,
    guarded_vectors: u64,
    guarded_controls: u64,
}
/// Checked String leaf effect; source permission still belongs to its exact execution operation.
#[derive(Clone, Copy)]
pub struct StringEffect<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    operation: LogicalOperation,
}
impl StringEffect<'_, '_> {
    /// Exact outer canonical entry; internal C delegation contributes no extra event.
    #[must_use]
    pub const fn operation(self) -> LogicalOperation {
        self.operation
    }
    /// Whether this effect belongs to the exact retaining new execution seal.
    #[must_use]
    pub fn belongs_to(self, owner: &VerifiedExecution<'_>) -> bool {
        std::ptr::eq(self.owner, owner)
    }
}
impl<'p, 'a> InvocationPlan<'p, 'a> {
    /// Fixed allocation-free invocation counter storage.
    #[must_use]
    pub const fn context_bytes(&self) -> u32 {
        32
    }
    /// Prepaid actual emitted String leaf call upper bound, including rollback paths.
    #[must_use]
    pub const fn guarded_string_sites(&self) -> u64 {
        self.guarded_strings
    }
    /// Prepaid finite emitted Vec call upper bound, including rollback storage release.
    #[must_use]
    pub const fn guarded_vector_sites(&self) -> u64 {
        self.guarded_vectors
    }
    /// Prepaid emitted control leaf sites, including canonical last-owner cleanup.
    #[must_use]
    pub const fn guarded_control_sites(&self) -> u64 {
        self.guarded_controls
    }
    /// Exact source operation effect, authenticated against this plan's owning program.
    /// # Errors
    /// Rejects a foreign view or a source operation with a different logical role.
    pub fn source_string(
        &self,
        op: VerifiedOperation<'_, '_>,
    ) -> Result<StringEffect<'p, 'a>, Diagnostic> {
        let site = op.site();
        let own = self
            .owner
            .functions()
            .nth(site.function)
            .and_then(|f| f.blocks().nth(site.block as usize))
            .and_then(|b| b.operations().nth(site.instruction as usize))
            .ok_or_else(invariant)?;
        if !std::ptr::eq(own.kind(), op.kind()) {
            return Err(invariant());
        }
        let operation = match own.kind() {
            raw::Operation::StringFromUtf8 { .. } => LogicalOperation::StringFromUtf8Copy,
            raw::Operation::StringConcat { .. } => LogicalOperation::StringConcat,
            _ => return Err(invariant()),
        };
        Ok(StringEffect { owner: self.owner, operation })
    }
}
impl<'p, 'a> VerifiedProgram<'p, 'a> {
    /// Exact String leaf of a checked Clone/drop recipe in this same program's layout world.
    /// This receipt cannot supply source permission or replace a Clone/drop receipt.
    /// # Errors
    /// Rejects a non-String layout or non-String leaf operation.
    pub fn string_leaf(&self, ty: u32, clone: bool) -> Result<StringEffect<'p, 'a>, Diagnostic> {
        if self.stored_type(ty).ok_or_else(invariant)?.category() != TypeCategory::String {
            return Err(invariant());
        }
        Ok(StringEffect {
            owner: self.program(),
            operation: if clone {
                LogicalOperation::StringClone
            } else {
                LogicalOperation::StringRelease
            },
        })
    }
}
pub(super) fn derive<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<InvocationPlan<'p, 'a>, Diagnostic> {
    let guarded_strings = prepay::derive(program, types, work)?;
    let guarded_vectors = prepay::vectors(program, types, work)?;
    let guarded_controls = prepay::controls(program, types, work)?;
    Ok(InvocationPlan { owner: program, guarded_strings, guarded_vectors, guarded_controls })
}
