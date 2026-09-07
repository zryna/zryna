//! Complete internal candidate commands; no public CLI profile is enabled here.

use zryna_abi::{Invocation, ScalarHostErrorCode, ScalarOutcome, ScalarTarget};
use zryna_diagnostics::Diagnostic;

use crate::{
    CommandFailure, CommandFailureKind, DataOwnershipBuildRequest, DataOwnershipRunRequest,
    NativeProcessLimits, OwnershipManifestResult, OwnershipTarget, PublishedOwnershipBundle,
    native::run_prepared_native_invocation,
    ownership_pipeline::{
        DataOwnershipCandidateSuccess, prepare_data_ownership_build, prepare_data_ownership_run,
    },
    ownership_publication::publish_after_staging,
    pipeline_runtime::{normalize_frame, render_javascript_harness, render_webassembly_harness},
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
            let harness = render_javascript_harness(success.artifact_stem(), &invocation)?;
            let harness_path = transaction.write_runtime_harness("javascript", &harness)?;
            let result_type = invocation.export().result();
            let carrier = node
                .run_javascript_module(&harness_path, transaction.path(), result_type)
                .map_err(execution_diagnostic)?;
            let outcome =
                match zryna_abi::normalize_result(ScalarTarget::JavaScript, result_type, carrier) {
                    Ok(value) => ScalarOutcome::Returned { value },
                    Err(_) => {
                        ScalarOutcome::HostError { code: ScalarHostErrorCode::InvalidTargetResult }
                    }
                };
            results.push(OwnershipManifestResult::new(OwnershipTarget::JavaScript, outcome));
        }
        if let Some(artifact) = artifacts.webassembly() {
            let harness = render_webassembly_harness(success.artifact_stem(), &invocation)?;
            let frame = node
                .run_webassembly_module(&harness, artifact.bytes(), transaction.path())
                .map_err(execution_diagnostic)?;
            results.push(OwnershipManifestResult::new(
                OwnershipTarget::WebAssembly,
                normalize_frame(ScalarTarget::CoreWebAssembly, invocation.export().result(), frame),
            ));
        }
        if let Some(executable) = artifacts.native_executable() {
            let outcome = run_prepared_native_invocation(
                executable.prepared_executable(),
                output,
                NativeProcessLimits::default(),
            )
            .map_err(|error| execution_diagnostic(error.diagnostic().clone()))?;
            results.push(OwnershipManifestResult::new(OwnershipTarget::Native, outcome));
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
