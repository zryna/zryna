//! Exact retained canonical declarations and metadata; no older layout identifiers.
use super::{Diagnostic, Model, StorageTarget, invariant};
use zryna_ownership_runtime_abi::{
    generic_v1::{ControlLayout, ElementLayout},
    raw::WebAssemblyLane as L,
};
pub(in crate::generic_execution_v1) const ALLOCATE_RAW: u32 = 0;
pub(in crate::generic_execution_v1) const RELEASE_RAW: u32 = 1;
pub(in crate::generic_execution_v1) const VEC_ALLOCATE: u32 = 2;
pub(in crate::generic_execution_v1) const VEC_RELEASE: u32 = 3;
pub(in crate::generic_execution_v1) const STRONG_CLONE: u32 = 4;
pub(in crate::generic_execution_v1) const DOWNGRADE: u32 = 5;
pub(in crate::generic_execution_v1) const WEAK_CLONE: u32 = 6;
pub(in crate::generic_execution_v1) const STRONG_BEGIN: u32 = 7;
pub(in crate::generic_execution_v1) const STRONG_FINISH: u32 = 8;
pub(in crate::generic_execution_v1) const WEAK_RELEASE: u32 = 9;
impl Model<'_, '_, '_> {
    pub(super) fn check_runtime_declarations(&self) -> Result<(), Diagnostic> {
        let decl = &self.program.runtime().declarations().webassembly;
        self.take(decl.len().checked_mul(17).ok_or_else(super::budget)?)?;
        let signatures = [
            ("allocate", 2, true),
            ("grow", 4, true),
            ("release", 3, false),
            ("stringFromUtf8Copy", 3, false),
            ("stringClone", 4, false),
            ("stringConcat", 7, false),
            ("stringRelease", 3, false),
            ("vecAllocate", 3, false),
            ("vecReserve", 6, false),
            ("vecReleaseStorage", 4, false),
            ("strongClone", 1, false),
            ("weakDowngrade", 1, false),
            ("weakClone", 1, false),
            ("weakUpgrade", 1, false),
            ("strongReleaseBegin", 2, false),
            ("strongReleaseFinish", 1, false),
            ("weakRelease", 2, false),
        ];
        if decl.len() != signatures.len() {
            return Err(invariant());
        }
        for (name, n, wide) in signatures {
            let d = decl.iter().find(|d| d.operation == name).ok_or_else(invariant)?;
            if d.parameters.len() != n
                || d.parameters.iter().any(|v| *v != L::I32)
                || d.results.as_slice() != [if wide { L::I64 } else { L::I32 }]
            {
                return Err(invariant());
            }
        }
        Ok(())
    }
    pub(in crate::generic_execution_v1) fn container(&self, role: u32) -> u32 {
        self.container_base + role
    }
    pub(in crate::generic_execution_v1) fn total_functions(&self) -> u32 {
        self.container_base + self.container_count
    }
    pub(in crate::generic_execution_v1) fn element(
        &self,
        ty: u32,
    ) -> Result<ElementLayout, Diagnostic> {
        let child = self.types[ty as usize].referenced_type().ok_or_else(invariant)?;
        let rows = self.program.runtime().element_layouts();
        self.take(rows.len())?;
        rows.iter()
            .copied()
            .find(|v| v.target() == StorageTarget::Linear32V1 && v.element() == child)
            .ok_or_else(invariant)
    }
    pub(in crate::generic_execution_v1) fn control(
        &self,
        ty: u32,
    ) -> Result<ControlLayout, Diagnostic> {
        let child = self.types[ty as usize].referenced_type().ok_or_else(invariant)?;
        let rows = self.program.runtime().control_layouts();
        self.take(rows.len())?;
        rows.iter()
            .copied()
            .find(|v| v.target() == StorageTarget::Linear32V1 && v.payload() == child)
            .ok_or_else(invariant)
    }
}
