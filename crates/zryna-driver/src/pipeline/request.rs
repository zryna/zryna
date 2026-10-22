//! Provider-independent command inputs and original source request validation.

use super::{
    BuildRequest, CommandFailure, CommandFailureKind, CommandKind, ControlFlowBuildRequest,
    TargetSelection, ValidatedRequest, entrypoint_error, failure, request_error,
    unsupported_component_request, validate_artifact_stem, validate_real_directory,
    validate_source_chain,
};
use crate::runtime::NodeRuntimeCapability;
use std::path::{Component, Path, PathBuf};

/// Provider-independent command inputs shared by the original publication path.
/// Runtime and frontend capabilities remain separate retained authorities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuildInputs {
    pub(crate) workspace_root: PathBuf,
    pub(crate) entrypoint: String,
    pub(crate) artifact_stem: String,
    pub(crate) targets: TargetSelection,
}

impl From<&BuildRequest> for BuildInputs {
    fn from(request: &BuildRequest) -> Self {
        Self {
            workspace_root: request.workspace_root.clone(),
            entrypoint: request.entrypoint.clone(),
            artifact_stem: request.artifact_stem.clone(),
            targets: request.targets,
        }
    }
}

impl From<&ControlFlowBuildRequest> for BuildInputs {
    fn from(request: &ControlFlowBuildRequest) -> Self {
        Self {
            workspace_root: request.workspace_root.clone(),
            entrypoint: request.entrypoint.clone(),
            artifact_stem: request.artifact_stem.clone(),
            targets: request.targets,
        }
    }
}

pub(super) fn validate_request(
    request: &BuildRequest,
    command: CommandKind,
) -> Result<ValidatedRequest, CommandFailure> {
    validate_build_inputs(&BuildInputs::from(request), command)
}

pub(super) fn validate_build_inputs(
    request: &BuildInputs,
    command: CommandKind,
) -> Result<ValidatedRequest, CommandFailure> {
    if !request.workspace_root.is_absolute() {
        return Err(request_error(
            "ZRYNA-C1001",
            "workspace root must be absolute",
            "resolve --root to an absolute real directory before dispatch",
        ));
    }
    if command == CommandKind::Run && request.targets.component() {
        return Err(unsupported_component_request(
            "component artifacts are build-only until a reviewed host profile is activated",
        ));
    }
    validate_artifact_stem(&request.artifact_stem)
        .map_err(|diagnostic| failure(CommandFailureKind::Request, diagnostic))?;
    if request.entrypoint.contains('\\') {
        return Err(entrypoint_error("entrypoint must use portable forward slashes"));
    }
    let has_noncanonical_segment = request
        .entrypoint
        .split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."));
    let entry = Path::new(&request.entrypoint);
    if has_noncanonical_segment
        || entry.is_absolute()
        || entry.extension().and_then(|value| value.to_str()) != Some("zry")
        || entry.components().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(entrypoint_error(
            "entrypoint must be one portable workspace-relative .zry path",
        ));
    }
    validate_real_directory(&request.workspace_root)
        .map_err(|diagnostic| failure(CommandFailureKind::Request, diagnostic))?;
    let source_path = request.workspace_root.join(entry);
    validate_source_chain(&request.workspace_root, &source_path)?;
    Ok(ValidatedRequest { source_path })
}

pub(crate) fn require_target_runtime(
    node: Option<&NodeRuntimeCapability>,
) -> Result<&NodeRuntimeCapability, CommandFailure> {
    node.ok_or_else(|| request_error("ZRYNA-C1001", "this target execution requires an explicit Node.js runtime", "pass --node with the documented direct runtime path; compilation and native execution do not require Node.js"))
}
