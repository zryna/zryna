//! Exact same-seal control effects; raw Allocate here is solely Shared control birth.
use super::{
    Diagnostic, LogicalOperation, TypeCategory, VerifiedExecution, VerifiedOperation,
    VerifiedProgram, invariant,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, VerifiedFunction};
/// Opaque checked control leaf role; metadata alone cannot construct an effect.
#[derive(Clone, Copy)]
pub struct ControlEffect<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
    operation: LogicalOperation,
    size: u64,
    alignment: u64,
}
impl ControlEffect<'_, '_> {
    /// Exact outer canonical operation; C factoring is not an extra event.
    #[must_use]
    pub const fn operation(self) -> LogicalOperation {
        self.operation
    }
    /// Exact retained native control allocation size.
    #[must_use]
    pub const fn size(self) -> u64 {
        self.size
    }
    /// Exact retained native control alignment.
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.alignment
    }
    /// Exact retaining new execution seal.
    #[must_use]
    pub fn belongs_to(self, owner: &VerifiedExecution<'_>) -> bool {
        std::ptr::eq(self.owner, owner)
    }
}
impl<'p, 'a> VerifiedProgram<'p, 'a> {
    fn control_effect(
        &self,
        ty: u32,
        operation: LogicalOperation,
    ) -> Result<ControlEffect<'p, 'a>, Diagnostic> {
        let layout = self.containers().and_then(|p| p.control(ty)).ok_or_else(invariant)?;
        Ok(ControlEffect {
            owner: self.program(),
            operation,
            size: layout.size(),
            alignment: layout.alignment(),
        })
    }
    /// Exact source Shared birth or Weak downgrade from this program's retained operation.
    /// # Errors
    /// Rejects foreign views and source carriers with another logical role.
    pub fn source_control(
        &self,
        op: VerifiedOperation<'_, '_>,
    ) -> Result<ControlEffect<'p, 'a>, Diagnostic> {
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
        if let Some(view) = own.shared_construct() {
            self.control_effect(view.ty(), LogicalOperation::Allocate)
        } else if let Some(view) = own.weak_downgrade() {
            self.control_effect(view.ty(), LogicalOperation::WeakDowngrade)
        } else {
            Err(invariant())
        }
    }
    /// Exact source upgrade terminator in the supplied opaque function/block pair.
    /// # Errors
    /// Rejects foreign/mismatched function or block views, and non-upgrade terminators.
    pub fn source_upgrade(
        &self,
        function: VerifiedFunction<'_, '_>,
        block: VerifiedBlock<'_, '_>,
    ) -> Result<ControlEffect<'p, 'a>, Diagnostic> {
        let own = self.program().functions().nth(function.index()).ok_or_else(invariant)?;
        if !std::ptr::eq(own.key(), function.key()) {
            return Err(invariant());
        }
        let actual = own.blocks().nth(block.id() as usize).ok_or_else(invariant)?;
        if !std::ptr::eq(actual.terminator(), block.terminator()) {
            return Err(invariant());
        }
        self.control_effect(
            actual.weak_upgrade().ok_or_else(invariant)?.weak_type(),
            LogicalOperation::WeakUpgrade,
        )
    }
    /// Exact Shared/Weak leaf of an existing checked Clone recipe.
    /// This receipt cannot replace the actual source Clone frontier.
    /// # Errors
    /// Rejects other categories or missing same-world control metadata.
    pub fn control_clone(&self, ty: u32) -> Result<ControlEffect<'p, 'a>, Diagnostic> {
        let operation = match self.stored_type(ty).ok_or_else(invariant)?.category() {
            TypeCategory::Shared => LogicalOperation::StrongClone,
            TypeCategory::Weak => LogicalOperation::WeakClone,
            _ => return Err(invariant()),
        };
        self.control_effect(ty, operation)
    }
    /// Exact control leaf of an existing checked drop recipe; Finish needs its last-Begin edge.
    /// This receipt supplies neither source drop permission nor a fabricated last-owner fact.
    /// # Errors
    /// Rejects non-control types or Finish on Weak.
    pub fn control_drop(&self, ty: u32, finish: bool) -> Result<ControlEffect<'p, 'a>, Diagnostic> {
        let operation = match (self.stored_type(ty).ok_or_else(invariant)?.category(), finish) {
            (TypeCategory::Shared, false) => LogicalOperation::StrongReleaseBegin,
            (TypeCategory::Shared, true) => LogicalOperation::StrongReleaseFinish,
            (TypeCategory::Weak, false) => LogicalOperation::WeakRelease,
            _ => return Err(invariant()),
        };
        self.control_effect(ty, operation)
    }
}
