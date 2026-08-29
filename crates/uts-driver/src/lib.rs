//! UTS compiler-phase orchestration.

#![forbid(unsafe_code)]

use std::path::Path;

use uts_architecture::ValidationReport;
use uts_backend_javascript::JavaScriptArtifact;
use uts_backend_native::LlvmIrArtifact;
use uts_diagnostics::Diagnostic;
use uts_ir::VerifiedProgram;

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
    uts_architecture::validate_workspace(root)
}

/// Emits both current backend artifacts from one verified program.
///
/// # Errors
///
/// Returns a backend diagnostic when either target cannot emit the verified program.
pub fn emit_verified(program: &VerifiedProgram) -> Result<DualTargetArtifacts, Diagnostic> {
    let javascript = uts_backend_javascript::emit(program)?;
    let mir = uts_native_mir::lower(program)?;
    let llvm_ir = uts_backend_native::emit_llvm_ir(&mir)?;
    Ok(DualTargetArtifacts { javascript, llvm_ir })
}
