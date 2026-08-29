//! Zryna compiler-phase orchestration.

#![forbid(unsafe_code)]

use std::path::Path;

use zryna_architecture::ValidationReport;
use zryna_backend_javascript::JavaScriptArtifact;
use zryna_backend_native::LlvmIrArtifact;
use zryna_diagnostics::Diagnostic;
use zryna_ir::VerifiedProgram;

/// Artifacts emitted by the first verified dual-target slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DualTargetArtifacts {
    /// Direct ECMAScript output.
    pub javascript: JavaScriptArtifact,
    /// Textual LLVM IR validating the native backend boundary.
    pub llvm_ir: LlvmIrArtifact,
}

/// Runs the mandatory workspace gate.
#[must_use]
pub fn check_workspace(root: &Path) -> ValidationReport {
    zryna_architecture::validate_workspace(root)
}

/// Emits both current backend artifacts from one verified program.
///
/// # Errors
///
/// Returns a backend diagnostic when either target cannot emit the verified program.
pub fn emit_verified(program: &VerifiedProgram) -> Result<DualTargetArtifacts, Diagnostic> {
    let javascript = zryna_backend_javascript::emit(program)?;
    let mir = zryna_native_mir::lower(program)?;
    let llvm_ir = zryna_backend_native::emit_llvm_ir(&mir)?;
    Ok(DualTargetArtifacts { javascript, llvm_ir })
}
