use std::time::Instant;

use zryna_frontend::VerifiedFrontendProviderV3;
use zryna_source::NormalizedSourcePath;

use super::{ModuleClosureError, VerifiedModuleClosure, discover_module_closure_with_clock};
use crate::WorkspaceSourceRoot;

/// Discovers, authenticates, and seals one bounded deterministic M2 module closure.
///
/// The provider receives only immutable source bytes and normalized portable paths. It never
/// receives the workspace capability or chooses a resolved host path. Intermediate snapshots are
/// discarded; only one final full-map snapshot is returned.
///
/// # Errors
///
/// Returns a fail-closed frontend or deterministic driver rejection before semantic analysis or
/// artifact creation.
pub fn discover_module_closure<Provider: VerifiedFrontendProviderV3 + ?Sized>(
    root: &WorkspaceSourceRoot,
    entrypoint: NormalizedSourcePath,
    frontend: &Provider,
) -> Result<VerifiedModuleClosure, ModuleClosureError> {
    discover_module_closure_with_clock(root, entrypoint, frontend, Instant::now)
}
