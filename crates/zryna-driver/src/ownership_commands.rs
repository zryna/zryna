//! Complete internal candidate commands; no public CLI profile is enabled here.

mod execution;

pub(crate) mod observation;
use zryna_diagnostics::Diagnostic;

use crate::{
    CommandFailure, CommandFailureKind, DataOwnershipBuildRequest, DataOwnershipRunRequest,
    PublishedOwnershipBundle,
    ownership_pipeline::{
        DataOwnershipCandidateSuccess, prepare_data_ownership_build, prepare_data_ownership_run,
    },
    ownership_publication::publish_after_staging,
    runtime::NodeRuntimeCapability,
};

/// Authenticates, prepares, and atomically publishes an internal candidate build.
///
/// This library-only entrypoint does not activate a public CLI profile.
///
/// # Errors
/// Returns phase-owned diagnostics and never advertises a partial bundle.
pub fn build_data_ownership_candidate(
    request: &DataOwnershipBuildRequest,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    let success = prepare_data_ownership_build(request)?;
    crate::ownership_publication::publish_data_ownership_build(&success)
}

/// Authenticates, executes, and atomically publishes an internal candidate run.
///
/// Every selected target executes from the same final syntax, semantic, layout, and runtime-ABI
/// authority. Publication begins only inside one private transaction.
///
/// # Errors
/// Returns source, backend, execution, transaction, or cleanup diagnostics without partial output.
pub fn run_data_ownership_candidate(
    request: DataOwnershipRunRequest,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    let success = prepare_data_ownership_run(request)?;
    execute_and_publish_run(&success)
}

fn execute_and_publish_run(
    success: &DataOwnershipCandidateSuccess,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    execute_with_fault(success, None)
}

fn execute_with_fault(
    success: &DataOwnershipCandidateSuccess,
    fault: Option<observation::Fault>,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    let run = execution::PreparedRun::new(success, fault)?;
    let node = NodeRuntimeCapability::discover(success.node_runtime(), success.workspace_root())
        .map_err(execution_diagnostic)?;
    publish_after_staging(success, |transaction, output| {
        run.execute(&node, transaction, output, &|| Ok(()))
    })
}

fn execution_diagnostic(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Execution, diagnostics: vec![diagnostic] }
}

pub(crate) fn execute_installed(
    prepared: &crate::ownership_pipeline::installed::PreparedInstalledOwnership<'_, '_>,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    prepared.revalidate()?;
    let run = execution::PreparedRun::new(prepared.success(), None)?;
    let node = prepared.admission().execution().node()?;
    crate::ownership_publication::publish_installed_after_staging(
        prepared,
        |transaction, output| run.execute(node, transaction, output, &|| prepared.revalidate()),
    )
}

fn execution_failure(code: &'static str, message: &'static str) -> CommandFailure {
    execution_diagnostic(Diagnostic::error(
        code,
        None,
        message,
        "retry the exact authenticated internal DataOwnershipV1 candidate request",
    ))
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;

#[cfg(test)]
mod conformance;

#[cfg(test)]
mod allocation_core;
