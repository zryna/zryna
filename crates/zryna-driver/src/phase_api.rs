//! Public verified frontend/semantic/backend phase orchestration, reexported unchanged at crate root.
use crate::{DualTargetArtifacts, SourceToIrError, SourceToIrSuccess};
use std::path::Path;
use zryna_architecture::ValidationReport;
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_ir::VerifiedProgram;
use zryna_source::SourceMap;

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
