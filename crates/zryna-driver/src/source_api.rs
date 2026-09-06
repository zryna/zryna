//! Public source-to-IR success and dual-target response models.

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

/// Successful source analysis with its verified IR and non-fatal provider diagnostics.
#[derive(Clone, Debug)]
pub struct SourceToIrSuccess {
    pub(crate) program: VerifiedProgram,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl SourceToIrSuccess {
    /// Returns the backend-safe verified program.
    #[must_use]
    pub const fn program(&self) -> &VerifiedProgram {
        &self.program
    }

    /// Consumes the result and returns the backend-safe verified program.
    #[must_use]
    pub fn into_program(self) -> VerifiedProgram {
        self.program
    }

    /// Returns deterministic non-fatal provider diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
