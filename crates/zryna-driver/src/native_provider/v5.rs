//! Typed V5 provider selection; comparison never substitutes one observation for the other.
use super::{NativeProviderCapability, arguments, identity_error};
use zryna_frontend::{
    ProviderExpectationV5, WorkerFrontendV5, WorkerLimitsV5, WorkerSpecV5,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
};

impl NativeProviderCapability {
    /// Creates an exact source-authenticating V5 worker without selecting a language profile.
    /// # Errors
    /// Rejects a stale current compiler identity or invalid bounded worker specification.
    pub fn frontend_v5(&self) -> Result<WorkerFrontendV5, zryna_diagnostics::Diagnostic> {
        self.revalidate()?;
        let expected = ProviderExpectationV5::new(NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION)
            .map_err(|_| identity_error())?;
        let spec = WorkerSpecV5::new(
            self.invocation.clone(),
            arguments(5),
            self.directory.clone(),
            expected,
            WorkerLimitsV5::default(),
        )
        .map_err(|_| identity_error())?;
        Ok(WorkerFrontendV5::new(spec))
    }
}

#[cfg(test)]
mod tests;
