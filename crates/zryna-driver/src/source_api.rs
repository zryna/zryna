//! Public source-to-IR response models.

use std::{error::Error, fmt};

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

/// Failure before source can become backend-safe verified IR.
#[derive(Debug)]
pub enum SourceToIrError {
    /// The authenticated frontend worker failed before returning verified syntax.
    Frontend(zryna_frontend::WorkerError),
    /// Provider, semantic, or IR diagnostics rejected the source.
    Rejected(Vec<Diagnostic>),
}

impl SourceToIrError {
    /// Returns source diagnostics when a compiler phase rejected the program.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Frontend(error) => error.diagnostics(),
            Self::Rejected(diagnostics) => diagnostics,
        }
    }
}

impl fmt::Display for SourceToIrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frontend(error) => error.fmt(formatter),
            Self::Rejected(diagnostics) => write!(
                formatter,
                "source was rejected by {} deterministic diagnostic(s)",
                diagnostics.len()
            ),
        }
    }
}

impl Error for SourceToIrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frontend(error) => Some(error),
            Self::Rejected(_) => None,
        }
    }
}
