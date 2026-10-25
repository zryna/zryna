//! Same-seal Vec leaf roles retain exact canonical element metadata.
use super::{
    Diagnostic, LogicalOperation, TypeCategory, VerifiedExecution, VerifiedOperation,
    VerifiedProgram, invariant,
};
/// Opaque Vec outer-event receipt; raw IDs and counter models cannot construct it.
#[derive(Clone, Copy)]
pub struct VectorEffect<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    operation: LogicalOperation,
    element: u32,
    stride: u64,
}
impl VectorEffect<'_, '_> {
    /// Exact outer canonical operation.
    #[must_use]
    pub const fn operation(self) -> LogicalOperation {
        self.operation
    }
    /// Exact same-layout element identity used by the canonical C declaration.
    #[must_use]
    pub const fn element(self) -> u32 {
        self.element
    }
    /// Checked physical element stride for the retained native target.
    #[must_use]
    pub const fn stride(self) -> u64 {
        self.stride
    }
    /// Whether the receipt belongs to this exact new execution seal.
    #[must_use]
    pub fn belongs_to(self, owner: &VerifiedExecution<'_>) -> bool {
        std::ptr::eq(self.owner, owner)
    }
}
impl<'p, 'a> VerifiedProgram<'p, 'a> {
    fn vector_effect(
        &self,
        ty: u32,
        operation: LogicalOperation,
    ) -> Result<VectorEffect<'p, 'a>, Diagnostic> {
        let metadata = self.containers().and_then(|p| p.element(ty)).ok_or_else(invariant)?;
        Ok(VectorEffect {
            owner: self.program(),
            operation,
            element: metadata.element().index(),
            stride: metadata.stride(),
        })
    }
    /// Exact source Vec construction/push, authenticated against this owning program.
    /// # Errors
    /// Rejects a foreign view or an operation without the requested source role.
    pub fn source_vector(
        &self,
        op: VerifiedOperation<'_, '_>,
    ) -> Result<VectorEffect<'p, 'a>, Diagnostic> {
        let site = op.site();
        let own = self
            .program()
            .functions()
            .nth(site.function)
            .and_then(|f| f.blocks().nth(site.block as usize))
            .and_then(|b| b.operations().nth(site.instruction as usize))
            .ok_or_else(invariant)?;
        if !std::ptr::eq(own.kind(), op.kind()) {
            return Err(invariant());
        }
        if let Some(view) = own.vec_construct() {
            self.vector_effect(view.ty(), LogicalOperation::VecAllocate)
        } else if let Some(view) = own.vec_push() {
            self.vector_effect(view.ty(), LogicalOperation::VecReserve)
        } else {
            Err(invariant())
        }
    }
    /// Exact Vec leaf of an existing checked Clone/drop recipe in this same layout world.
    /// This receipt cannot supply source Clone/drop permission.
    /// # Errors
    /// Rejects non-Vec layouts and missing checked element metadata.
    pub fn vector_leaf(&self, ty: u32, clone: bool) -> Result<VectorEffect<'p, 'a>, Diagnostic> {
        if self.stored_type(ty).ok_or_else(invariant)?.category() != TypeCategory::Vec {
            return Err(invariant());
        }
        self.vector_effect(
            ty,
            if clone { LogicalOperation::VecAllocate } else { LogicalOperation::VecReleaseStorage },
        )
    }
}
