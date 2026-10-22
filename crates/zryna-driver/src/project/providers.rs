//! Admission bridges from legacy and explicit provider command requests.

use super::{
    CommandFailure, PackageResolutionRequest, ProjectAdmission, ProjectBuildRequest,
    RootPackageSources,
};

impl ProjectAdmission {
    pub(super) fn discover_profile(
        request: &ProjectBuildRequest,
        profile: &str,
        resolve: fn(
            &PackageResolutionRequest,
        ) -> Result<
            (crate::PackageResolutionSuccess, RootPackageSources),
            zryna_package::ResolveError,
        >,
    ) -> Result<Self, CommandFailure> {
        Self::discover_inputs(
            &request.compiler_root,
            &crate::pipeline::BuildInputs {
                workspace_root: request.project_root.clone(),
                entrypoint: request.entrypoint.clone(),
                artifact_stem: request.artifact_stem.clone(),
                targets: request.targets,
            },
            profile,
            resolve,
        )
    }

    pub(crate) fn discover_provider(
        compiler_root: &std::path::Path,
        request: &crate::pipeline::BuildInputs,
        profile: &str,
        retain: bool,
    ) -> Result<Self, CommandFailure> {
        Self::discover_inputs(
            compiler_root,
            request,
            profile,
            if retain { RootPackageSources::retained } else { RootPackageSources::snapshot },
        )
    }
}
