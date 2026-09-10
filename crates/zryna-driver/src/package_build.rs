//! Authenticated pure-source package build-plan execution.

mod cache;
mod plan;
mod publication;
mod staging;

#[cfg(test)]
mod tests;

use std::{error::Error, fmt, path::PathBuf};

use crate::{ArtifactOutputRoot, PackageResolutionSuccess};
use serde::Serialize;

pub use cache::ArtifactCacheRoot;
pub use plan::{
    BuildCompilerIdentity, BuildCompositionIdentity, BuildEnvironmentEntry, BuildHostIdentity,
    BuildHostToolIdentity, BuildOutput, BuildPlanIdentity, BuildProfileIdentity,
    BuildRuntimeIdentity, BuildTarget, BuildTargetId, ExecutionPolicyIdentity,
};

/// Exact package-build manifest filename.
pub const PACKAGE_BUILD_MANIFEST_NAME: &str = "zryna-package-build-manifest-v1.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Network and lock behavior already enforced while admitting a package build.
pub enum PackageBuildMode {
    /// Inputs were admitted without network access.
    Offline,
    /// The exact existing lock and all inputs were admitted without network access.
    Frozen,
}

#[derive(Clone, Debug)]
/// Explicit authenticated identity inputs for one pure-source build plan.
pub struct BuildPlanConfiguration {
    /// Exact compiler identity.
    pub compiler: BuildCompilerIdentity,
    /// Exact language-profile identity and configuration.
    pub profile: BuildProfileIdentity,
    /// Opaque accepted execution-policy identity.
    pub execution_policy: ExecutionPolicyIdentity,
    /// Declared host and output-relevant environment.
    pub host: BuildHostIdentity,
    /// Complete sorted selected target set covered by the package graph.
    pub targets: Vec<BuildTarget>,
    /// Complete sorted host-tool inventory.
    pub host_tools: Vec<BuildHostToolIdentity>,
    /// Complete sorted final output inventory.
    pub outputs: Vec<BuildOutput>,
}

#[derive(Debug)]
/// Driver request for deterministic package-plan execution and publication.
pub struct PackageBuildRequest<'a> {
    /// Authenticated package graph and immutable source snapshot.
    pub resolution: &'a PackageResolutionSuccess,
    /// Explicit output-relevant plan identities.
    pub configuration: BuildPlanConfiguration,
    /// Offline or exact-lock frozen admission mode.
    pub mode: PackageBuildMode,
    /// Retained project-owned artifact cache.
    pub cache_root: &'a ArtifactCacheRoot,
    /// Retained project-owned output root.
    pub output_root: &'a ArtifactOutputRoot,
}

/// One dependency result supplied only to the package compiler for this build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledDependency<'a> {
    /// Exact dependency package manifest identity.
    pub package_id: &'a str,
    /// Opaque deterministic compiler result for the dependency.
    pub bytes: &'a [u8],
}

/// One authenticated package compilation unit.
#[derive(Clone, Debug)]
pub struct PackageCompilationUnit<'a> {
    /// Exact package manifest identity.
    pub package_id: &'a str,
    /// Domain-separated complete source inventory digest.
    pub source_sha256: &'a str,
    /// Immutable authenticated source files in path order.
    pub sources: &'a [crate::AuthenticatedPackageFile],
    /// Direct dependencies in canonical alias order.
    pub dependencies: Vec<CompiledDependency<'a>>,
    /// Exact target identity for this compilation.
    pub target: &'a BuildTarget,
    /// Whether this unit is the only package allowed to return final outputs.
    pub is_root: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One exact declared root output produced by the pure-source compiler.
pub struct CompiledOutput {
    /// Exact target-qualified path declared by the build plan.
    pub path: String,
    /// Complete deterministic output bytes.
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Result of compiling one package for one target.
pub struct PackageCompilationResult {
    /// Opaque deterministic bytes available to dependent package compilations.
    pub dependency_bytes: Vec<u8>,
    /// Final outputs; non-root packages must return an empty collection.
    pub outputs: Vec<CompiledOutput>,
}

/// Structurally valid cached outputs presented to the trusted compiler for authentication.
#[derive(Clone, Debug)]
pub struct CachedTargetAuthentication<'a> {
    /// Complete authenticated package graph and immutable source snapshot.
    pub resolution: &'a PackageResolutionSuccess,
    /// Exact canonical plan that addressed the cache entry.
    pub plan: &'a BuildPlanIdentity,
    /// Exact selected target.
    pub target: &'a BuildTarget,
    /// Cached outputs after closed metadata, inventory, size, and digest validation.
    pub outputs: &'a [CompiledOutput],
}

/// Explicit in-process pure-source compiler owned by the driver caller.
///
/// Package source can never select an executable or process through this interface.
pub trait PureSourceCompiler {
    /// Compiles one authenticated package unit without package-selected host operations.
    ///
    /// # Errors
    ///
    /// Returns a package-build error when the explicit compiler cannot produce the unit.
    fn compile(
        &mut self,
        unit: PackageCompilationUnit<'_>,
    ) -> Result<PackageCompilationResult, PackageBuildError>;

    /// Authenticates cached output bytes against trusted compiler/source authority.
    ///
    /// Cache metadata and its self-declared hashes are not sufficient authority. Implementations
    /// may deterministically reproduce the outputs or verify a compiler-owned attestation that is
    /// bound to the supplied plan, sources, target, and bytes.
    ///
    /// # Errors
    ///
    /// Returns a compiler-boundary failure when the cached bytes cannot be authenticated.
    fn authenticate_cached_outputs(
        &mut self,
        authentication: CachedTargetAuthentication<'_>,
    ) -> Result<(), PackageBuildError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Whether one target was compiled or reused from the authenticated cache.
pub enum TargetCacheOutcome {
    /// A complete authenticated entry was reused without compilation.
    Hit,
    /// No addressed entry existed, so the target was compiled and committed.
    Miss,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One published target observation.
pub struct PublishedPackageTarget {
    /// Exact target.
    pub target: BuildTargetId,
    /// Domain-separated target cache key.
    pub target_cache_key: String,
    /// Validated hit or newly compiled miss.
    pub cache: TargetCacheOutcome,
    /// Complete final output inventory.
    pub outputs: Vec<PublishedPackageOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
/// One output in the committed package bundle.
pub struct PublishedPackageOutput {
    /// Exact target-qualified output path.
    pub path: String,
    /// Output byte length.
    pub bytes: u64,
    /// Raw output SHA-256 digest.
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Complete create-only package build publication.
pub struct PackageBuildSuccess {
    plan: BuildPlanIdentity,
    manifest_path: PathBuf,
    targets: Vec<PublishedPackageTarget>,
}

impl PackageBuildSuccess {
    #[must_use]
    /// Returns the exact canonical plan and its cache identities.
    pub fn plan(&self) -> &BuildPlanIdentity {
        &self.plan
    }

    #[must_use]
    /// Returns the committed package build manifest path.
    pub fn manifest_path(&self) -> &std::path::Path {
        &self.manifest_path
    }

    #[must_use]
    /// Returns target observations in canonical target order.
    pub fn targets(&self) -> &[PublishedPackageTarget] {
        &self.targets
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Stable fail-closed package build rejection.
pub struct PackageBuildError {
    code: &'static str,
    detail: String,
}

impl PackageBuildError {
    #[must_use]
    /// Constructs an explicit compiler-boundary failure.
    pub fn compiler(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-B4104", detail: detail.into() }
    }

    pub(crate) fn plan(detail: impl Into<String>) -> Self {
        Self { code: "ZRYNA-B4101", detail: detail.into() }
    }

    pub(crate) fn cache(detail: &str) -> Self {
        Self { code: "ZRYNA-B4102", detail: detail.to_owned() }
    }

    pub(crate) fn publication(detail: &str) -> Self {
        Self { code: "ZRYNA-B4103", detail: detail.to_owned() }
    }

    #[must_use]
    /// Returns the stable diagnostic code.
    pub fn code(&self) -> &'static str {
        self.code
    }

    #[must_use]
    /// Returns the bounded non-path diagnostic detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for PackageBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl Error for PackageBuildError {}

/// Constructs the canonical plan, reuses or fills its deterministic cache, and publishes one
/// complete create-only output bundle.
///
/// # Errors
///
/// Returns a stable plan, cache, compiler, or publication failure without reporting a partial
/// bundle as successful.
pub fn execute_package_build(
    request: &PackageBuildRequest<'_>,
    compiler: &mut impl PureSourceCompiler,
) -> Result<PackageBuildSuccess, PackageBuildError> {
    let prepared = plan::prepare(request)?;
    let materialized = cache::execute_targets(request, &prepared, compiler)?;
    let manifest_path = publication::publish(request, &prepared, &materialized)?;
    let targets = materialized.into_iter().map(|target| target.observation).collect();
    Ok(PackageBuildSuccess { plan: prepared.identity, manifest_path, targets })
}
