use std::path::PathBuf;

use crate::{CommandFailure, ProjectBuildRequest, TargetSelection, project::ProjectAdmission};

use super::{InstalledCompiler, InstalledExecution, InstalledProjectSources};

/// Existing compiler profile selected for an installed project request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstalledProfile {
    /// Default M1 scalar profile.
    I32,
    /// Existing M2 control-flow profile.
    ControlFlow,
    /// Existing M3 data-ownership profile.
    DataOwnership,
}

impl InstalledProfile {
    pub(crate) const fn package_profile(self) -> &'static str {
        match self {
            Self::I32 => "i32-v1",
            Self::ControlFlow => "control-flow-v1",
            Self::DataOwnership => "data-ownership-v1",
        }
    }
}

/// Project-owned inputs for an installed build; no compiler root or runtime is caller-selected.
#[derive(Clone, Debug)]
pub struct InstalledBuildRequest {
    /// Explicit absolute project root.
    pub project_root: PathBuf,
    /// Exact declared root-package entrypoint.
    pub entrypoint: String,
    /// Portable output artifact stem.
    pub artifact_stem: String,
    /// Existing target selection.
    pub targets: TargetSelection,
    /// Existing profile, matching the frozen package compatibility record.
    pub profile: InstalledProfile,
}

pub(crate) struct InstalledAdmission<'installation> {
    project: ProjectAdmission,
    execution: InstalledExecution<'installation>,
    request: ProjectBuildRequest,
}

impl<'installation> InstalledAdmission<'installation> {
    pub(crate) fn prepare(
        installation: &'installation InstalledCompiler,
        request: &InstalledBuildRequest,
    ) -> Result<Self, CommandFailure> {
        let project_request = ProjectBuildRequest {
            compiler_root: installation.root.clone(),
            project_root: request.project_root.clone(),
            entrypoint: request.entrypoint.clone(),
            artifact_stem: request.artifact_stem.clone(),
            targets: request.targets,
            node_runtime: installation.root.join(installation.distribution.node_path()),
        };
        let project =
            ProjectAdmission::discover_installed(&project_request, installation, request.profile)?;
        let execution = InstalledExecution::prepare(installation)?;
        let admission = Self { project, execution, request: project_request };
        admission.revalidate()?;
        Ok(admission)
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandFailure> {
        self.project.revalidate()?;
        self.execution.revalidate()
    }

    pub(crate) fn execution(&self) -> &InstalledExecution<'installation> {
        &self.execution
    }
    pub(crate) fn project(&self) -> &ProjectAdmission {
        &self.project
    }
    pub(crate) fn request(&self) -> &ProjectBuildRequest {
        &self.request
    }

    pub(crate) fn sources(&self) -> InstalledProjectSources<'_, 'installation> {
        InstalledProjectSources::new(&self.project, &self.execution)
    }
}
