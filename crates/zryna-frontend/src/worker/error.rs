//! Closed frontend-worker failure construction and observation.
use zryna_diagnostics::Diagnostic;

use super::{WorkerError, WorkerFailure};

impl WorkerError {
    /// Rejects divergent independently verified provider observations before semantic dispatch.
    ///
    /// This creates only a failure; it cannot construct or authenticate a syntax snapshot.
    #[must_use]
    pub fn provider_divergence() -> Self {
        Self::snapshot_verification(vec![Diagnostic::error(
            "ZRYNA-F1107",
            None,
            "native and bootstrap frontend observations differ",
            "retain the exact source and report the provider comparison failure",
        )])
    }

    pub(super) const fn new(failure: WorkerFailure) -> Self {
        Self { failure, diagnostics: Vec::new() }
    }

    pub(super) fn snapshot_verification(diagnostics: Vec<Diagnostic>) -> Self {
        Self { failure: WorkerFailure::SnapshotVerification, diagnostics }
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn failure(&self) -> WorkerFailure {
        self.failure
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.failure.code()
    }

    /// Returns bounded structural diagnostics when snapshot verification failed.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
