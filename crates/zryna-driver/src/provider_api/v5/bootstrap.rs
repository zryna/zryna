//! Exact admitted Node and compiler-owned TypeScript V5 worker configuration.
use crate::{
    CommandFailure,
    provider_api::preparation_failure,
    runtime::{NodeRuntimeCapability, node_compatible_path},
};
use std::{fs, path::Path};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::{ProviderExpectationV5, WorkerFrontendV5, WorkerLimitsV5, WorkerSpecV5};

pub(super) fn configured(
    root: &Path,
    node: &NodeRuntimeCapability,
) -> Result<WorkerFrontendV5, CommandFailure> {
    node.revalidate().map_err(preparation_failure)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| unavailable())?;
    if !metadata.is_dir() || link_like(&metadata) {
        return Err(unavailable());
    }
    let worker = root.join("src/worker-v5.mjs");
    let metadata = fs::symlink_metadata(&worker).map_err(|_| unavailable())?;
    if !metadata.is_file() || link_like(&metadata) {
        return Err(unavailable());
    }
    let expected = ProviderExpectationV5::new("typescript-6", "6.0.3")
        .map_err(|error| worker_failure(&error))?;
    let spec = WorkerSpecV5::new(
        node.executable().map_err(preparation_failure)?,
        vec![node_compatible_path(&worker).into_os_string()],
        node_compatible_path(root),
        expected,
        WorkerLimitsV5::default(),
    )
    .map_err(|error| worker_failure(&error))?;
    Ok(WorkerFrontendV5::new(spec))
}
fn worker_failure(error: &zryna_frontend::WorkerError) -> CommandFailure {
    if error.diagnostics().is_empty() {
        return preparation_failure(Diagnostic::error(
            error.code(),
            None,
            error.to_string(),
            "retain the exact bounded V5 worker and explicit Node capability",
        ));
    }
    CommandFailure {
        kind: crate::CommandFailureKind::Preparation,
        diagnostics: error.diagnostics().to_vec(),
    }
}
fn unavailable() -> CommandFailure {
    preparation_failure(Diagnostic::error(
        "ZRYNA-F1107",
        None,
        "typed V5 bootstrap worker is not an ordinary compiler-owned file",
        "retain the pinned compiler checkout and explicit Node host",
    ))
}
#[cfg(windows)]
fn link_like(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
