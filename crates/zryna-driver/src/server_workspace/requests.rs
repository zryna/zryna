//! Source and listener authority stay independent of optional syntax runtimes.

use std::path::PathBuf;

/// Explicit source-checkout selection; no installed-package or ambient guest authority.
#[derive(Clone)]
pub struct ServerRunRequest<Node = PathBuf> {
    /// Absolute compiler workspace root.
    pub workspace_root: PathBuf,
    /// One portable workspace-relative source.
    pub entrypoint: String,
    /// Sole no-argument i32 source export.
    pub export: String,
    /// Fresh portable output stem.
    pub artifact_stem: String,
    /// Absolute pinned Node syntax-provider executable.
    pub node_runtime: Node,
    /// Absolute caller-private configuration file.
    pub configuration: PathBuf,
    /// Separate absolute caller-private root listener approval.
    pub listener_approval: PathBuf,
}

/// The distinct one-read clock arrangement with a separate private guest request and approval.
#[derive(Clone)]
pub struct ClockServerRunRequest<Node = PathBuf> {
    /// Existing authenticated source and configuration-bound listener selection.
    pub server: ServerRunRequest<Node>,
    /// Absolute caller-private exact one-read guest request document.
    pub guest_request: PathBuf,
    /// Absolute separate caller-private root approval bound to exact request bytes.
    pub guest_approval: PathBuf,
}

impl ServerRunRequest {
    pub(super) fn optional_runtime(&self) -> ServerRunRequest<Option<PathBuf>> {
        ServerRunRequest {
            workspace_root: self.workspace_root.clone(),
            entrypoint: self.entrypoint.clone(),
            export: self.export.clone(),
            artifact_stem: self.artifact_stem.clone(),
            node_runtime: Some(self.node_runtime.clone()),
            configuration: self.configuration.clone(),
            listener_approval: self.listener_approval.clone(),
        }
    }
}
