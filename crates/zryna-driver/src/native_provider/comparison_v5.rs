//! Private typed V5 comparison; two physical analyses, complete files and advisories.
use std::time::{Duration, Instant};
use zryna_frontend::{
    MAX_WORKER_TIMEOUT, MIN_WORKER_TIMEOUT, VerifiedFrontendProviderV5, WorkerError,
    WorkerFrontendV5,
    native_worker::{NATIVE_PROVIDER, NATIVE_PROVIDER_VERSION},
    syntax_v5,
};
use zryna_source::SourceMap;
pub(crate) struct ComparisonFrontendV5 {
    native: WorkerFrontendV5,
    bootstrap: WorkerFrontendV5,
}
impl ComparisonFrontendV5 {
    pub(crate) const fn new(native: WorkerFrontendV5, bootstrap: WorkerFrontendV5) -> Self {
        Self { native, bootstrap }
    }
}
impl VerifiedFrontendProviderV5 for ComparisonFrontendV5 {
    fn analysis_work_units(&self) -> usize {
        2
    }
    fn minimum_analysis_timeout(&self) -> Duration {
        MIN_WORKER_TIMEOUT * 2
    }
    fn analyze_verified_v5(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v5::VerifiedProjectSyntaxV5, WorkerError> {
        self.analyze_verified_v5_with_timeout(sources, MAX_WORKER_TIMEOUT)
    }
    fn analyze_verified_v5_with_timeout(
        &self,
        sources: &SourceMap,
        timeout: Duration,
    ) -> Result<syntax_v5::VerifiedProjectSyntaxV5, WorkerError> {
        let native = self.native.spec();
        let bootstrap = self.bootstrap.spec();
        if native.expected().provider() != NATIVE_PROVIDER
            || native.expected().provider_version() != NATIVE_PROVIDER_VERSION
            || bootstrap.expected().provider() != "typescript-6"
            || bootstrap.expected().provider_version() != "6.0.3"
        {
            return Err(WorkerError::provider_divergence());
        }
        let budget = timeout
            .min(MAX_WORKER_TIMEOUT)
            .min(native.limits().timeout())
            .min(bootstrap.limits().timeout());
        if budget < self.minimum_analysis_timeout() {
            return Err(WorkerError::provider_divergence());
        }
        let started = Instant::now();
        let first = self.native.analyze_verified_v5_with_timeout(sources, budget / 2)?;
        let remaining =
            budget.checked_sub(started.elapsed()).ok_or_else(WorkerError::provider_divergence)?;
        let second = self.bootstrap.analyze_verified_v5_with_timeout(sources, remaining)?;
        equal(sources, first, &second)
    }
}

fn equal(
    sources: &SourceMap,
    first: syntax_v5::VerifiedProjectSyntaxV5,
    second: &syntax_v5::VerifiedProjectSyntaxV5,
) -> Result<syntax_v5::VerifiedProjectSyntaxV5, WorkerError> {
    if !first.is_bound_to(sources)
        || !second.is_bound_to(sources)
        || first.files() != second.files()
        || first.advisories() != second.advisories()
    {
        return Err(WorkerError::provider_divergence());
    }
    Ok(first)
}

#[cfg(test)]
mod tests;
