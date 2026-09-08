//! Complete internal candidate commands; no public CLI profile is enabled here.

use zryna_abi::Invocation;

pub(crate) mod observation;
use zryna_diagnostics::Diagnostic;

use crate::{
    CommandFailure, CommandFailureKind, DataOwnershipBuildRequest, DataOwnershipRunRequest,
    NativeProcessLimits, OwnershipTarget, PublishedOwnershipBundle,
    native::ownership::observation::run,
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
    let export = success.logical_export().ok_or_else(|| {
        execution_failure("ZRYNA-C3301", "candidate run lost its authenticated invocation identity")
    })?;
    let invocation = success
        .program()
        .verified_ir()
        .scalar_abi()
        .prepare_invocation(Invocation::new(export.to_owned(), success.arguments().to_vec()))
        .map_err(|error| {
            execution_failure(
                error.code(),
                "candidate run invocation no longer matches its verified scalar ABI",
            )
        })?;
    let node = NodeRuntimeCapability::discover(success.node_runtime(), success.workspace_root())
        .map_err(execution_diagnostic)?;
    publish_after_staging(success, |transaction, output| {
        let mut results = Vec::with_capacity(3);
        let artifacts = success.artifacts();
        if artifacts.javascript().is_some() {
            let harness = observation::javascript(success.artifact_stem(), &invocation, fault)?;
            let harness_path = transaction.write_runtime_harness("javascript", &harness)?;
            let frame = node
                .run_ownership_javascript(&harness_path, transaction.path())
                .map_err(execution_diagnostic)?;
            results.push(observation::decode(
                &frame,
                invocation.export().result(),
                OwnershipTarget::JavaScript,
            )?);
        }
        if let Some(artifact) = artifacts.webassembly() {
            let harness = observation::webassembly(&invocation, fault)?;
            let frame = node
                .run_ownership_webassembly(&harness, artifact.bytes(), transaction.path())
                .map_err(execution_diagnostic)?;
            results.push(observation::decode(
                &frame,
                invocation.export().result(),
                OwnershipTarget::WebAssembly,
            )?);
        }
        if let Some(executable) = artifacts.native_executable() {
            let frame = run(
                executable.prepared_executable(),
                output,
                NativeProcessLimits::default(),
                fault.map(observation::Fault::command),
            )
            .map_err(|error| execution_diagnostic(error.diagnostic().clone()))?;
            results.push(observation::decode(
                &frame,
                invocation.export().result(),
                OwnershipTarget::Native,
            )?);
        }
        node.revalidate().map_err(execution_diagnostic)?;
        Ok(results)
    })
}

fn execution_diagnostic(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Execution, diagnostics: vec![diagnostic] }
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
