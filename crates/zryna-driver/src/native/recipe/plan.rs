//! Private source/FFI-bound compile request. This module performs no host effects.
use super::super::native_c_v0::resource_identity::HandleLinkRequirements;
use crate::package_build::{
    BuildPlanConfiguration, BuildPlanIdentity, BuildTargetId, PackageBuildMode,
};
use crate::{PackageBuildError, PackageResolutionSuccess};
use serde_json::Value;
use std::collections::BTreeMap;
use zryna_source::SourceMap;

mod ffi;
mod identity;
mod materials;

/// In-memory references use exactly the existing source compileInput terms.
#[derive(Clone, Debug)]
pub struct SourceInput {
    /// Exact resolved package identity.
    pub package_id: String,
    /// Exact resolved source-snapshot digest.
    pub source_sha256: String,
    /// Portable path in the retained package inventory.
    pub path: String,
}
/// Read-only sysroot data, not a host path or permission to execute its files.
#[derive(Clone, Debug)]
pub struct ReadOnlyMaterial {
    /// Explicit recipe sysroot identity.
    pub id: String,
    /// Complete retained path-to-byte inventory; no ambient filesystem paths.
    pub files: BTreeMap<String, Vec<u8>>,
}
/// One bounded raw typed request; only prepare constructs a retained plan.
#[derive(Clone, Debug)]
pub struct NativeCompileStep {
    /// Explicit compile step identity.
    pub id: String,
    /// Exact selected name in the approved host tool inventory.
    pub tool: String,
    /// Exact retained package source input.
    pub source: SourceInput,
    /// Complete independently approved read-only material.
    pub sysroots: Vec<ReadOnlyMaterial>,
    /// Logical declared output identity.
    pub output: String,
}
/// Effect-free raw request; retained source and ABI issuers are independently checked.
pub struct CompilePlanRequest<'a> {
    /// Original authenticated package-resolution result.
    pub resolution: &'a PackageResolutionSuccess,
    /// Exact independently selected build identities and declared environment.
    pub configuration: &'a BuildPlanConfiguration,
    /// Existing offline or frozen lock behavior.
    pub mode: PackageBuildMode,
    /// One declarative compile-only step.
    pub step: NativeCompileStep,
    /// Package containing the complete original Zryna source subset.
    pub ffi_package_id: &'a str,
    /// Original source map retained by the ABI issuer.
    pub sources: &'a SourceMap,
    /// Original verified source-to-machine link requirements; hashes cannot construct these.
    pub requirements: &'a HandleLinkRequirements,
}
/// No raw record can construct this retained source/ABI identity.
#[derive(Debug)]
pub struct NativeCompilePlan {
    resolution: PackageResolutionSuccess,
    configuration: BuildPlanConfiguration,
    source_plan: BuildPlanIdentity,
    mode: PackageBuildMode,
    step: NativeCompileStep,
    source_bytes: Vec<u8>,
    sources: SourceMap,
    requirements: HandleLinkRequirements,
    recipe_sha256: String,
    sysroot_inventory: Vec<Value>,
    ffi_package_id: String,
}
#[derive(Debug)]
#[non_exhaustive]
/// Native recipe admission, object audit or execution refusal.
pub enum NativeRecipeError {
    /// An existing source-plan owner rejected its inputs.
    Plan(PackageBuildError),
    /// The existing complete foreign-object audit rejected its inputs.
    Object(zryna_diagnostics::Diagnostic),
    /// Cleanup is unconfirmed; retained owner identity permits trusted recovery.
    Cleanup {
        /// Original operation failure, if any; a captured object is never returned here.
        primary: Option<Box<NativeRecipeError>>,
        /// Fresh ownership label used only for this attempted invocation.
        owner: String,
        /// Fixed cleanup refusal description, without ambient host secrets.
        detail: &'static str,
    },
    /// A recipe-specific boundary rejected the operation.
    Rejected {
        /// Stable refusal category.
        reason: &'static str,
        /// Fixed diagnostic description, without ambient host secrets.
        detail: &'static str,
    },
}
impl NativeRecipeError {
    /// Stable refusal category from the owning boundary.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::Plan(error) => error.code(),
            Self::Object(error) => error.code(),
            Self::Rejected { reason, .. } => reason,
            Self::Cleanup { .. } => "TRUST-CLEANUP",
        }
    }
    /// Diagnostic description from the owning boundary.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Plan(error) => error.detail(),
            Self::Object(error) => error.message(),
            Self::Rejected { detail, .. } | Self::Cleanup { detail, .. } => detail,
        }
    }
}
impl std::fmt::Display for NativeRecipeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.reason(), self.detail())
    }
}
impl std::error::Error for NativeRecipeError {}
pub(super) fn denied(reason: &'static str, detail: &'static str) -> NativeRecipeError {
    NativeRecipeError::Rejected { reason, detail }
}
pub(super) fn prepare(
    request: CompilePlanRequest<'_>,
) -> Result<NativeCompilePlan, NativeRecipeError> {
    let source_plan = crate::package_build::prepare_source_plan_identity(
        request.resolution,
        request.configuration,
        request.mode,
    )
    .map_err(NativeRecipeError::Plan)?;
    let configuration = request.configuration;
    if configuration.host.triple != "x86_64-unknown-linux-gnu"
        || configuration.targets.len() != 1
        || configuration.targets[0].id != BuildTargetId::NativeLinuxX86_64
        || configuration.targets[0].triple != "x86_64-unknown-linux-gnu"
        || configuration.outputs.len() != 1
        || !materials::name(&request.step.id)
        || !materials::name(&request.step.output)
    {
        return Err(denied(
            "TRUST-NATIVE",
            "only one explicit Linux native compile output is admitted",
        ));
    }
    if !configuration.host_tools.iter().any(|tool| {
        tool.name == request.step.tool
            && tool.runs_on == configuration.host.triple
            && tool.targets.contains(&BuildTargetId::NativeLinuxX86_64)
    }) {
        return Err(denied(
            "TRUST-TOOL",
            "compile tool is absent from the exact host tool inventory",
        ));
    }
    ffi::check(&request)?;
    let package = request
        .resolution
        .sources()
        .iter()
        .find(|package| {
            package.package_id() == request.step.source.package_id
                && package.source_sha256() == request.step.source.source_sha256
        })
        .ok_or_else(|| denied("TRUST-SOURCE", "compile source package identity differs"))?;
    let file =
        package.files().iter().find(|file| file.path() == request.step.source.path).ok_or_else(
            || denied("TRUST-SOURCE", "compile source is absent from the retained inventory"),
        )?;
    if file.path().rsplit_once('.').is_none_or(|(_, extension)| extension != "c") {
        return Err(denied("TRUST-NATIVE", "compile input must be one declared C source"));
    }
    let source_bytes = file.bytes().to_vec();
    let sysroot_inventory = materials::sysroots(&request.step)?;
    let recipe_sha256 = identity::recipe(&request, &source_plan, &sysroot_inventory)?;
    Ok(NativeCompilePlan {
        resolution: request.resolution.clone(),
        configuration: configuration.clone(),
        source_plan,
        mode: request.mode,
        step: request.step,
        source_bytes,
        sources: request.sources.clone(),
        requirements: request.requirements.clone(),
        recipe_sha256,
        sysroot_inventory,
        ffi_package_id: request.ffi_package_id.to_owned(),
    })
}
impl NativeCompilePlan {
    /// Original authenticated resolution retained by this plan.
    #[must_use]
    pub fn resolution(&self) -> &PackageResolutionSuccess {
        &self.resolution
    }
    /// Exact source-plan configuration; changing a clone cannot change this plan.
    #[must_use]
    pub fn configuration(&self) -> &BuildPlanConfiguration {
        &self.configuration
    }
    /// Existing canonical source-plan identity, without cache or publication authority.
    #[must_use]
    pub fn source_plan(&self) -> &BuildPlanIdentity {
        &self.source_plan
    }
    /// Existing offline or frozen lock behavior retained at preparation.
    #[must_use]
    pub fn mode(&self) -> PackageBuildMode {
        self.mode
    }
    /// Exact immutable declarative compile step.
    #[must_use]
    pub fn step(&self) -> &NativeCompileStep {
        &self.step
    }
    /// Exact retained compile input; no ambient file is reopened.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
    /// Original source map authenticated by the retained ABI issuer.
    #[must_use]
    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }
    /// Original opaque source-to-machine link requirements.
    #[must_use]
    pub fn requirements(&self) -> &HandleLinkRequirements {
        &self.requirements
    }
    /// Internal exact compile-plan identity to compare with independent approval.
    #[must_use]
    pub fn recipe_sha256(&self) -> &str {
        &self.recipe_sha256
    }
    /// Exact retained readonly material inventory, bound into the recipe identity.
    #[must_use]
    pub fn sysroot_inventory(&self) -> &[Value] {
        &self.sysroot_inventory
    }
    /// Exact package containing the source-map-bound FFI declarations.
    #[must_use]
    pub fn ffi_package_id(&self) -> &str {
        &self.ffi_package_id
    }
    /// Exact declared output path; this grants no filesystem publication permission.
    #[must_use]
    pub fn output_path(&self) -> &str {
        &self.configuration.outputs[0].path
    }
}
