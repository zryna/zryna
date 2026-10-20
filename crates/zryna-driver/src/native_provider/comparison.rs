//! Transition comparison of two independently verified observations over the same source map.

use std::time::{Duration, Instant};

use zryna_frontend::{
    MAX_WORKER_TIMEOUT, MIN_WORKER_TIMEOUT, VerifiedFrontendProvider, VerifiedFrontendProviderV3,
    VerifiedFrontendProviderV4, WorkerError, WorkerFrontend, WorkerFrontendV3, WorkerFrontendV4,
    WorkerLimits, WorkerSpec,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
    syntax_v2, syntax_v3, syntax_v4,
};
use zryna_source::SourceMap;

/// Explicit v2 transition comparison. A divergence never falls back to either provider.
pub struct ComparisonFrontendV2 {
    native: WorkerFrontend,
    bootstrap: WorkerFrontend,
}
impl ComparisonFrontendV2 {
    /// Combines exact native and bootstrap configurations without granting syntax authority.
    #[must_use]
    pub const fn new(native: WorkerFrontend, bootstrap: WorkerFrontend) -> Self {
        Self { native, bootstrap }
    }
}
impl VerifiedFrontendProvider for ComparisonFrontendV2 {
    fn analyze_verified(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v2::ProjectSyntaxSnapshot, WorkerError> {
        let native = self.native.spec();
        let bootstrap = self.bootstrap.spec();
        identities(
            native.expected().provider(),
            native.expected().provider_version(),
            bootstrap.expected().provider(),
            bootstrap.expected().provider_version(),
        )?;
        let budget =
            native.limits().timeout().min(bootstrap.limits().timeout()).min(MAX_WORKER_TIMEOUT);
        let started = Instant::now();
        let first = tighten_v2(&self.native, budget / 2)?.analyze_verified(sources)?;
        let remaining =
            budget.checked_sub(started.elapsed()).ok_or_else(WorkerError::provider_divergence)?;
        let second = tighten_v2(&self.bootstrap, remaining)?.analyze_verified(sources)?;
        equal(first, &second)
    }
}

fn tighten_v2(worker: &WorkerFrontend, timeout: Duration) -> Result<WorkerFrontend, WorkerError> {
    let spec = worker.spec();
    let original = spec.limits();
    let limits = WorkerLimits::new(
        timeout.min(original.timeout()),
        original.stdout_bytes(),
        original.stderr_bytes(),
    )?;
    Ok(WorkerFrontend::new(WorkerSpec::new(
        spec.executable(),
        spec.arguments().to_vec(),
        spec.current_dir(),
        spec.expected().clone(),
        limits,
    )?))
}

/// Explicit v3 transition comparison preserving one caller-tightened whole comparison budget.
pub struct ComparisonFrontendV3 {
    native: WorkerFrontendV3,
    bootstrap: WorkerFrontendV3,
}
impl ComparisonFrontendV3 {
    /// Combines exact native and bootstrap configurations without granting syntax authority.
    #[must_use]
    pub const fn new(native: WorkerFrontendV3, bootstrap: WorkerFrontendV3) -> Self {
        Self { native, bootstrap }
    }
}
impl VerifiedFrontendProviderV3 for ComparisonFrontendV3 {
    fn analysis_work_units(&self) -> usize {
        2
    }

    fn minimum_analysis_timeout(&self) -> Duration {
        MIN_WORKER_TIMEOUT * 2
    }
    fn analyze_verified_v3(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, WorkerError> {
        self.analyze_verified_v3_with_timeout(sources, MAX_WORKER_TIMEOUT)
    }
    fn analyze_verified_v3_with_timeout(
        &self,
        sources: &SourceMap,
        timeout: Duration,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, WorkerError> {
        let native = self.native.spec();
        let bootstrap = self.bootstrap.spec();
        identities(
            native.expected().provider(),
            native.expected().provider_version(),
            bootstrap.expected().provider(),
            bootstrap.expected().provider_version(),
        )?;
        let budget = timeout
            .min(MAX_WORKER_TIMEOUT)
            .min(native.limits().timeout())
            .min(bootstrap.limits().timeout());
        let started = Instant::now();
        let first = self.native.analyze_verified_v3_with_timeout(sources, budget / 2)?;
        let remaining =
            budget.checked_sub(started.elapsed()).ok_or_else(WorkerError::provider_divergence)?;
        let second = self.bootstrap.analyze_verified_v3_with_timeout(sources, remaining)?;
        equal(first, &second)
    }
}

/// Explicit v4 transition comparison preserving one caller-tightened whole comparison budget.
pub struct ComparisonFrontendV4 {
    native: WorkerFrontendV4,
    bootstrap: WorkerFrontendV4,
}
impl ComparisonFrontendV4 {
    /// Combines exact native and bootstrap configurations without granting syntax authority.
    #[must_use]
    pub const fn new(native: WorkerFrontendV4, bootstrap: WorkerFrontendV4) -> Self {
        Self { native, bootstrap }
    }
}
impl VerifiedFrontendProviderV4 for ComparisonFrontendV4 {
    fn analysis_work_units(&self) -> usize {
        2
    }

    fn minimum_analysis_timeout(&self) -> Duration {
        MIN_WORKER_TIMEOUT * 2
    }
    fn analyze_verified_v4(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        self.analyze_verified_v4_with_timeout(sources, MAX_WORKER_TIMEOUT)
    }
    fn analyze_verified_v4_with_timeout(
        &self,
        sources: &SourceMap,
        timeout: Duration,
    ) -> Result<syntax_v4::ProjectSyntaxSnapshot, WorkerError> {
        let native = self.native.spec();
        let bootstrap = self.bootstrap.spec();
        identities(
            native.expected().provider(),
            native.expected().provider_version(),
            bootstrap.expected().provider(),
            bootstrap.expected().provider_version(),
        )?;
        let budget = timeout
            .min(MAX_WORKER_TIMEOUT)
            .min(native.limits().timeout())
            .min(bootstrap.limits().timeout());
        let started = Instant::now();
        let first = self.native.analyze_verified_v4_with_timeout(sources, budget / 2)?;
        let remaining =
            budget.checked_sub(started.elapsed()).ok_or_else(WorkerError::provider_divergence)?;
        let second = self.bootstrap.analyze_verified_v4_with_timeout(sources, remaining)?;
        equal(first, &second)
    }
}

fn identities(
    native: &str,
    native_version: &str,
    bootstrap: &str,
    bootstrap_version: &str,
) -> Result<(), WorkerError> {
    if native != NATIVE_PROVIDER
        || native_version != NATIVE_PROVIDER_VERSION
        || bootstrap != "typescript-6"
        || bootstrap_version != "6.0.3"
    {
        return Err(WorkerError::provider_divergence());
    }
    Ok(())
}

fn equal<T: Eq>(native: T, bootstrap: &T) -> Result<T, WorkerError> {
    if &native != bootstrap {
        return Err(WorkerError::provider_divergence());
    }
    Ok(native)
}
