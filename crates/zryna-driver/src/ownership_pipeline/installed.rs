//! Installed M3 dispatch keeps its installation/project proof alive through publication.

use zryna_abi::ScalarValue;
use zryna_source::NormalizedSourcePath;

use crate::{
    CommandFailure, DataOwnershipBuildRequest, PublishedOwnershipBundle,
    distribution::{
        InstalledAdmission, InstalledBuildRequest, InstalledCompiler, InstalledProfile,
    },
};

use super::{
    DataOwnershipCandidateSuccess, closure_failure, preparation, request_error,
    validate_request_shape,
};

pub(crate) struct PreparedInstalledOwnership<'proof, 'installation> {
    success: DataOwnershipCandidateSuccess,
    admission: &'proof InstalledAdmission<'installation>,
}

impl<'installation> PreparedInstalledOwnership<'_, 'installation> {
    pub(crate) fn success(&self) -> &DataOwnershipCandidateSuccess {
        &self.success
    }
    pub(crate) fn admission(&self) -> &InstalledAdmission<'installation> {
        self.admission
    }
    pub(crate) fn revalidate(&self) -> Result<(), CommandFailure> {
        self.admission.revalidate()
    }
}

pub(crate) fn build(
    installation: &InstalledCompiler,
    request: &InstalledBuildRequest,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    require_profile(request)?;
    let admission = InstalledAdmission::prepare(installation, request)?;
    let prepared = prepare(&admission, None)?;
    crate::ownership_publication::publish_installed_build(&prepared)
}

pub(crate) fn run(
    installation: &InstalledCompiler,
    request: &InstalledBuildRequest,
    export: String,
    arguments: Vec<ScalarValue>,
) -> Result<PublishedOwnershipBundle, CommandFailure> {
    require_profile(request)?;
    let admission = InstalledAdmission::prepare(installation, request)?;
    let prepared = prepare(&admission, Some((export, arguments)))?;
    crate::ownership_commands::execute_installed(&prepared)
}

fn require_profile(request: &InstalledBuildRequest) -> Result<(), CommandFailure> {
    if request.profile == InstalledProfile::DataOwnership {
        Ok(())
    } else {
        Err(request_error("installed ownership route requires data-ownership-v1"))
    }
}

fn prepare<'proof, 'installation>(
    admission: &'proof InstalledAdmission<'installation>,
    run: Option<(String, Vec<ScalarValue>)>,
) -> Result<PreparedInstalledOwnership<'proof, 'installation>, CommandFailure> {
    let project = admission.request();
    let request = DataOwnershipBuildRequest {
        workspace_root: project.project_root.clone(),
        entrypoint: project.entrypoint.clone(),
        artifact_stem: project.artifact_stem.clone(),
        targets: project.targets,
        node_runtime: project.node_runtime.clone(),
    };
    validate_request_shape(&request)?;
    let path = NormalizedSourcePath::new(request.entrypoint.clone())
        .map_err(|error| request_error(error.to_string()))?;
    let frontend = admission.execution().frontend_v4()?;
    let closure = crate::ownership_closure::discover_with_clock(
        &admission.sources(),
        path,
        &frontend,
        std::time::Instant::now,
    )
    .map_err(|error| closure_failure(&error))?;
    let success =
        preparation::prepare_closure(&request, closure, run, &|_| admission.revalidate(), &|| {
            admission.revalidate()
        })?;
    admission.revalidate()?;
    Ok(PreparedInstalledOwnership { success, admission })
}
