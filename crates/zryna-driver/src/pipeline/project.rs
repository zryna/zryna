//! Standalone-project dispatch into the default-profile pipeline.

use super::{
    CommandFailure, CommandSuccess, RunInvocation, execute_after_architecture,
    validate_architecture,
};
use crate::{ProjectBuildRequest, ProjectRunRequest, project::ProjectAdmission};

pub(crate) fn build_project_request(
    project: &ProjectBuildRequest,
) -> Result<CommandSuccess, CommandFailure> {
    validate_architecture(&project.compiler_root)?;
    let admission = ProjectAdmission::discover(project)?;
    execute_after_architecture(
        &project.as_workspace_request(),
        None,
        &project.compiler_root,
        Some(&admission),
    )
}

pub(crate) fn run_project_request(
    request: ProjectRunRequest,
) -> Result<CommandSuccess, CommandFailure> {
    validate_architecture(&request.build.compiler_root)?;
    let admission = ProjectAdmission::discover(&request.build)?;
    let invocation =
        RunInvocation { logical_export: request.logical_export, arguments: request.arguments };
    execute_after_architecture(
        &request.build.as_workspace_request(),
        Some(&invocation),
        &request.build.compiler_root,
        Some(&admission),
    )
}
