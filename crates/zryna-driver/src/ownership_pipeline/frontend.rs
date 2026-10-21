//! Explicit legacy bootstrap frontend admission.

#[cfg(test)]
use super::test_support;
use super::{
    CommandFailure, CommandFailureKind, DataOwnershipBuildRequest, NodeRuntimeCapability, failure,
    is_link_like, preparation_error,
};
use crate::runtime::node_compatible_path;
use std::fs;
use zryna_frontend::{ProviderExpectationV4, WorkerFrontendV4, WorkerLimitsV4, WorkerSpecV4};

pub(super) fn configured_frontend_v4(
    request: &DataOwnershipBuildRequest,
    node: &NodeRuntimeCapability,
) -> Result<WorkerFrontendV4, CommandFailure> {
    let adapter = request.workspace_root.join("adapters/typescript-6");
    configured_frontend_at(&adapter, node)
}

#[cfg(test)]
pub(super) fn configured_test_frontend_v4(
    _request: &DataOwnershipBuildRequest,
    node: &NodeRuntimeCapability,
) -> Result<WorkerFrontendV4, CommandFailure> {
    configured_frontend_at(&test_support::adapter_root(), node)
}

pub(crate) fn configured_frontend_at(
    adapter: &std::path::Path,
    node: &NodeRuntimeCapability,
) -> Result<WorkerFrontendV4, CommandFailure> {
    let worker = adapter.join("src/worker-v4.mjs");
    for (path, label) in [(adapter, "adapter directory"), (worker.as_path(), "worker entrypoint")] {
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| preparation_error(format!("DataOwnershipV1 {label} is unavailable")))?;
        if (path == adapter && !metadata.is_dir())
            || (path == worker && !metadata.is_file())
            || is_link_like(&metadata)
        {
            return Err(preparation_error(format!("DataOwnershipV1 {label} is not a real path")));
        }
    }
    let expected =
        ProviderExpectationV4::new("typescript-6", "6.0.3").map_err(|error| CommandFailure {
            kind: CommandFailureKind::Preparation,
            diagnostics: error.diagnostics().to_vec(),
        })?;
    let spec = WorkerSpecV4::new(
        node.executable().map_err(|item| failure(CommandFailureKind::Preparation, item))?,
        vec![node_compatible_path(&worker).into_os_string()],
        node_compatible_path(adapter),
        expected,
        WorkerLimitsV4::default(),
    )
    .map_err(|error| CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: error.diagnostics().to_vec(),
    })?;
    Ok(WorkerFrontendV4::new(spec))
}
