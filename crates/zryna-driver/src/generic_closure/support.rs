//! Existing module diagnostic/resource categories without an older syntax/program seal.
use super::{
    BTreeMap, CommandFailure, DiscoveryFailure, Duration, Instant, ModuleClosureError,
    NormalizedSourcePath, SourceMap, syntax_v5,
};
use crate::{CommandFailureKind, source_session::ModuleSourceSession};
use zryna_diagnostics::{Diagnostic, Severity};

pub(super) fn diagnostic(
    code: &'static str,
    path: Option<&NormalizedSourcePath>,
    message: impl Into<String>,
) -> Diagnostic {
    let message = message.into();
    Diagnostic::error(
        code,
        None,
        path.map_or(message.clone(), |path| format!("{}: {message}", path.as_str())),
        "retain one unchanged bounded source closure and exact V5 provider",
    )
}
pub(super) fn rejected(diagnostic: Diagnostic) -> ModuleClosureError {
    ModuleClosureError::Rejected(vec![diagnostic])
}
pub(super) fn invariant() -> ModuleClosureError {
    rejected(diagnostic("ZRYNA-D3102", None, "generic source closure binding invariant failed"))
}
pub(super) fn budget(message: &'static str) -> ModuleClosureError {
    rejected(diagnostic("ZRYNA-D3201", None, message))
}
pub(super) fn add(left: usize, right: usize) -> Result<usize, ModuleClosureError> {
    left.checked_add(right).ok_or_else(|| budget("generic module accounting overflow"))
}
pub(super) fn source_failure(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Source, diagnostics: vec![diagnostic] }
}
pub(super) fn failure(error: &ModuleClosureError) -> CommandFailure {
    let diagnostics = match error {
        ModuleClosureError::Frontend(worker) if worker.diagnostics().is_empty() => {
            vec![Diagnostic::error(
                worker.code(),
                None,
                worker.to_string(),
                "retain the selected exact V5 provider and bounded runtime",
            )]
        }
        _ => error.diagnostics().to_vec(),
    };
    CommandFailure { kind: CommandFailureKind::Source, diagnostics }
}
pub(super) fn checkpoint(
    session: &mut impl ModuleSourceSession,
    sources: &SourceMap,
    outer: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<(), DiscoveryFailure> {
    outer().map_err(DiscoveryFailure::Checkpoint)?;
    session.validate_provider_batch(sources).map_err(rejected)?;
    session.revalidate_all().map_err(rejected)?;
    Ok(())
}
pub(super) fn snapshot(
    snapshot: &syntax_v5::VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<(), ModuleClosureError> {
    if !snapshot.is_bound_to(sources) {
        return Err(invariant());
    }
    if snapshot.advisories().iter().any(|item| item.severity == Severity::Error) {
        return Err(rejected(diagnostic(
            "ZRYNA-D3101",
            None,
            "exact V5 provider rejected the source batch",
        )));
    }
    Ok(())
}
pub(super) fn remaining(
    started: Instant,
    now: Instant,
    minimum: Duration,
    limit: Duration,
) -> Result<Duration, ModuleClosureError> {
    let elapsed = now.checked_duration_since(started).ok_or_else(invariant)?;
    let remaining = limit
        .checked_sub(elapsed)
        .ok_or_else(|| budget("generic module discovery wall time exceeded"))?;
    if remaining < minimum {
        return Err(budget("generic module discovery lacks a worker cleanup reserve"));
    }
    Ok(remaining)
}
pub(super) fn portable(
    paths: &mut BTreeMap<String, NormalizedSourcePath>,
    path: &NormalizedSourcePath,
) -> Result<(), ModuleClosureError> {
    let identity = path.portable_identity();
    if paths.get(&identity).is_some_and(|other| other != path) {
        return Err(rejected(diagnostic(
            "ZRYNA-D3005",
            Some(path),
            "portable module path collision",
        )));
    }
    paths.insert(identity, path.clone());
    Ok(())
}
pub(super) fn target(
    path: &NormalizedSourcePath,
    specifier: &str,
) -> Result<NormalizedSourcePath, ModuleClosureError> {
    zryna_source::resolve_explicit_zry_import(path, specifier).map_err(|_| {
        rejected(diagnostic(
            "ZRYNA-D3001",
            Some(path),
            "module import requires one explicit in-root relative .zry path",
        ))
    })
}
