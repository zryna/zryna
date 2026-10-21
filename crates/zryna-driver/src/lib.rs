//! Zryna compiler-phase orchestration.
//!
//! Legacy protocol-v1 syntax cannot enter the protocol-v2 semantic boundary:
//!
//! ```compile_fail
//! fn bypass(
//!     legacy: &zryna_frontend::ProjectSyntaxSnapshot,
//!     sources: &zryna_source::SourceMap,
//! ) {
//!     let _ = zryna_semantics::SemanticInput::try_new(legacy, sources);
//! }
//! ```
//!
//! Private scalar adapter authorities are intentionally absent from the external crate surface:
//!
//! ```compile_fail
//! let _ = zryna_driver::VerifiedScalarEsm;
//! ```
//!
//! ```compile_fail
//! fn accept_raw(_: zryna_driver::scalar_adapter_interface::raw::Interface) {}
//! ```

#![forbid(unsafe_code)]

mod browser_component;
mod command_h1_runtime;
mod command_h1_workspace;
mod command_request;
mod command_runtime;
pub mod diagnostic_sessions;
pub mod distribution;
mod javascript;
mod module_closure;
#[cfg(test)]
mod module_closure_tests;
mod native;
mod native_provider;
mod ownership_api;
mod ownership_closure;
mod ownership_commands;
mod ownership_manifest;
mod ownership_pipeline;
mod ownership_publication;
mod ownership_runtime_v1;
mod package_build;
mod package_resolution;
mod pipeline;
mod pipeline_runtime;
#[cfg(any(unix, test))]
mod process_spawn;
mod profile_composition;
mod project;
mod provider_api;
mod restricted_browser;
mod runtime;
mod scalar_adapter_interface;
mod server_lifecycle;
mod server_runtime;
mod server_transport;
mod server_workspace;
mod source_api;
mod source_graph_identity;
mod source_identity;
mod source_session;
mod webassembly;
mod workspace_source;

use std::path::Path;

use zryna_architecture::ValidationReport;
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::VerifiedProgram;
use zryna_source::SourceMap;

pub use command_h1_runtime::{
    COMMAND_H1_MANIFEST_NAME, COMMAND_H1_MANIFEST_SCHEMA, CommandH1ExecutionRecord,
    CommandH1HostPolicy, CommandH1Manifest, CommandH1Outcome, CommandH1RunReturn,
    CommandH1Teardown, CommandH1TrapCategory, ExecutedCommandH1, MAX_COMMAND_H1_MANIFEST_BYTES,
    PreparedCommandH1, decode_command_h1_manifest, prepare_command_h1,
};
pub use command_h1_workspace::{
    CommandH1RunRequest, PublishedCommandH1Bundle, run_command_h1_with_provider,
    run_command_h1_workspace,
};
pub use command_runtime::{CommandHostPolicy, PreparedCommand, prepare_command_self_check};
pub use javascript::{
    ArtifactOutputRoot, JAVASCRIPT_ARTIFACT_EXTENSION, JavaScriptBuildError,
    JavaScriptBuildSuccess, JavaScriptOutputRoot, MAX_ARTIFACT_STEM_BYTES,
    MAX_JAVASCRIPT_ARTIFACT_STEM_BYTES, PublishedJavaScriptArtifact, compile_javascript,
    publish_javascript,
};
pub use module_closure::entry::discover_native_straight_line_closure;
pub use module_closure::native_sources::{
    NativeModuleSnapshot, NativeOwnershipSnapshot, NativeSourceSnapshot, NativeSyntaxSnapshot,
    capture_native_package_sources, capture_native_workspace_sources,
};
pub use module_closure::{
    MAX_MODULE_DIRECTORY_ENTRIES, MAX_MODULE_DISCOVERY_ROUNDS, MAX_MODULE_DISCOVERY_WALL_TIME,
    MAX_MODULE_EDGE_MANIFEST_BYTES, MAX_MODULE_FILES, MAX_MODULE_IMPORT_DECLARATIONS,
    MAX_MODULE_IMPORT_EDGES, MAX_MODULE_PROVIDER_CALLS, MAX_MODULE_PROVIDER_SOURCE_BYTES,
    MAX_MODULE_SOURCE_BYTES, ModuleClosureError, ModuleEdge, ModuleRecord, VerifiedModuleClosure,
    discover_module_closure, discover_native_import_only_closure,
};
pub use native::recipe::{
    ApprovedNativeRecipe, NativeCompilePlan, NativeCompileRecipeRequest, NativeCompileRecipeStep,
    NativeCompileSourceInput, NativeRecipeError, NativeRecipeOperatorApproval,
    NativeRecipeReadOnlyMaterial, NativeRecipeRequirements, approve_native_recipe,
    prepare_and_approve_native_recipe, prepare_native_recipe,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use native::recipe::{
    NativeRecipeSuccess, NativeToolRootApproval, execute_native_recipe, native_recipe_environment,
    native_recipe_policy_identity, native_tool_root_identity,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use native::recipe::{NativeToolRootEntry, NativeToolRootKind, NativeToolRootMetadata};
pub use native::{
    LinuxX8664LinkToolchain, MAX_NATIVE_EXECUTABLE_BYTES, MAX_NATIVE_LINK_TIMEOUT,
    MAX_NATIVE_OBJECT_ARTIFACT_STEM_BYTES, MAX_NATIVE_PROBE_TIMEOUT, MAX_NATIVE_RUN_STDERR_BYTES,
    MAX_NATIVE_RUN_TIMEOUT, MAX_NATIVE_TOOL_OUTPUT_BYTES, NATIVE_EXECUTABLE_ARTIFACT_EXTENSION,
    NATIVE_OBJECT_ARTIFACT_EXTENSION, NativeExecutableBuildError, NativeExecutableBuildSuccess,
    NativeObjectBuildError, NativeObjectBuildSuccess, NativeObjectOutputRoot, NativeProcessLimits,
    NativeRunError, PublishedNativeExecutableArtifact, PublishedNativeObjectArtifact,
    compile_native_invocation, compile_native_object, discover_linux_native_toolchain, native_c_v0,
    publish_native_object, run_native_invocation, select_native_object_target,
};
pub use ownership_api::*;
pub use package_build::*;
pub use package_resolution::{
    AuthenticatedPackageFile, AuthenticatedPackageSources, PackageLockMode,
    PackageResolutionRequest, PackageResolutionSuccess, resolve_package,
};
pub use pipeline::{
    BuildRequest, CommandFailure, CommandFailureKind, CommandKind, CommandSuccess,
    ControlFlowBuildRequest, ControlFlowRunRequest, PublishedTargetArtifact, RunRequest,
    TargetResult, TargetSelection, build_control_flow_workspace, build_workspace,
    run_control_flow_workspace, run_workspace,
};
pub use project::{ProjectBuildRequest, ProjectRunRequest, build_project, run_project};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use restricted_browser::daemon::{
    PlaygroundDaemon, PlaygroundDaemonError, PlaygroundDaemonMaterialPin,
    PlaygroundDaemonOperatorApproval, PlaygroundDaemonResponse, approve_playground_daemon,
    playground_daemon_policy_sha256,
};
pub use restricted_browser::{
    BROWSER_SOURCE_PATH, BrowserCompilation, MAX_BROWSER_SOURCE_BYTES, RestrictedBrowserCompiler,
    RestrictedBrowserError, restrict_browser_runtime,
};
#[cfg(test)]
use server_lifecycle::{Input, Limits};
#[cfg(test)]
use server_runtime::test_support::{Resource, attach, input, limits};
pub use server_workspace::{
    ClockServerRunRequest, PublishedServerBundle, ServerReadiness, ServerRunRequest,
    serve_clock_workspace, serve_clock_workspace_with_provider, serve_workspace,
    serve_workspace_with_provider,
};
pub use source_api::{DualTargetArtifacts, SourceToIrError, SourceToIrSuccess};
pub use webassembly::{
    MAX_WEBASSEMBLY_ARTIFACT_STEM_BYTES, PublishedWebAssemblyArtifact,
    WEBASSEMBLY_ARTIFACT_EXTENSION, WebAssemblyBuildError, WebAssemblyBuildSuccess,
    WebAssemblyOutputRoot, compile_webassembly, publish_webassembly,
};
pub use workspace_source::WorkspaceSourceRoot;

/// Runs the mandatory workspace gate.
#[must_use]
pub fn check_workspace(root: &Path) -> ValidationReport {
    zryna_architecture::validate_workspace(root)
}

/// Runs one authenticated frontend worker and returns only source-map-verified syntax.
///
/// # Errors
///
/// Returns a deterministic worker failure before untrusted syntax can enter later phases.
pub fn analyze_sources<Provider: zryna_frontend::VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<zryna_frontend::syntax_v2::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
    frontend.analyze_verified(sources)
}

/// Lowers one source-map-bound protocol-v2 snapshot and runs the mandatory IR verifier.
///
/// Provider errors stop before semantic analysis. Provider warnings remain observable on success.
///
/// # Errors
///
/// Returns deterministic diagnostics and never exposes raw IR when semantic or IR verification
/// fails.
pub fn lower_verified_syntax(
    syntax: &zryna_frontend::syntax_v2::ProjectSyntaxSnapshot,
    sources: &SourceMap,
) -> Result<SourceToIrSuccess, Vec<Diagnostic>> {
    if !syntax.is_bound_to(sources) {
        return Err(vec![Diagnostic::error(
            "ZRYNA-D1001",
            None,
            "verified syntax is not bound to the driver's authoritative source map",
            "analyze and lower with the same immutable source map instance",
        )]);
    }
    if syntax.diagnostics().iter().any(|diagnostic| diagnostic.severity() == Severity::Error) {
        return Err(syntax.diagnostics().to_vec());
    }
    let Some(input) = zryna_semantics::SemanticInput::try_new(syntax, sources) else {
        return Err(vec![Diagnostic::error(
            "ZRYNA-D1002",
            None,
            "verified syntax could not enter semantic analysis",
            "report this compiler invariant failure with the smallest reproducible source",
        )]);
    };
    let program = zryna_semantics::lower(input)?;
    let program = zryna_ir::verify(program, sources)?;
    let diagnostics = syntax
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Warning)
        .cloned()
        .collect();
    Ok(SourceToIrSuccess { program, diagnostics })
}

/// Authenticates a frontend, analyzes real source, lowers strict semantics, and verifies IR.
///
/// # Errors
///
/// Returns the exact frontend failure or bounded rejection diagnostics. No backend can observe an
/// unverified program through this path.
pub fn compile_to_verified_ir<Provider: zryna_frontend::VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &SourceMap,
) -> Result<SourceToIrSuccess, SourceToIrError> {
    let syntax = analyze_sources(frontend, sources).map_err(SourceToIrError::Frontend)?;
    lower_verified_syntax(&syntax, sources).map_err(SourceToIrError::Rejected)
}

/// Emits both current backend artifacts from one verified program.
///
/// Raw Universal IR cannot enter driver emission:
///
/// ```compile_fail
/// let raw = zryna_ir::Program::default();
/// let _ = zryna_driver::emit_verified(&raw);
/// ```
///
/// # Errors
///
/// Returns bounded diagnostics when native MIR verification or either target emission fails.
pub fn emit_verified(program: &VerifiedProgram) -> Result<DualTargetArtifacts, Vec<Diagnostic>> {
    let javascript = zryna_backend_javascript::emit(program).map_err(|error| vec![error])?;
    let mir = zryna_native_mir::lower(program)?;
    let llvm_ir = zryna_backend_native::emit_llvm_ir(&mir).map_err(|error| vec![error])?;
    Ok(DualTargetArtifacts { javascript, llvm_ir })
}

#[cfg(test)]
#[path = "runtime_boundary_tests/fuel_meter.rs"]
mod runtime_boundary_tests;
#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "owned_sanitizer_runtime_fixture.rs"]
mod owned_sanitizer_runtime_fixture;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[doc(hidden)]
pub use owned_sanitizer_runtime_fixture::m5_owned_sanitizer_runtime;

pub use native_provider::NativeProviderCapability;

pub use provider_api::{
    CompilationProfile, CompilationSource, CompilerProvider, ProviderBuildRequest,
    ProviderCommandSuccess, build_with_provider, run_with_provider,
};

/// Serves exactly one fixed native-frontend handshake and source request.
///
/// # Errors
/// Rejects unsupported protocol versions, malformed framing and bounded source failures.
pub fn run_native_frontend_worker(protocol: u32) -> Result<(), Diagnostic> {
    zryna_frontend::native_worker::run_native_worker_stdio(protocol).map_err(|error| {
        Diagnostic::error(
            "ZRYNA-F1107",
            None,
            error.to_string(),
            "use the compiler-owned native frontend transport",
        )
    })
}
