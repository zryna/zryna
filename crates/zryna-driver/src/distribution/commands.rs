//! Public installed commands require an admitted compiler and project-only request.

use super::{InstalledBuildRequest, InstalledCompiler, InstalledProfile};
use crate::{CommandFailure, CommandSuccess, PublishedOwnershipBundle};

/// Existing versioned command output selected by the project profile.
pub enum InstalledCommandSuccess {
    /// Default scalar or control-flow bundle.
    Scalar(CommandSuccess),
    /// Data-ownership bundle.
    Ownership(PublishedOwnershipBundle),
}

impl InstalledCompiler {
    /// Builds a project using only this installation's authenticated runtime and provider.
    ///
    /// # Errors
    /// Rejects changed installation/project proofs and existing compiler or publication errors.
    pub fn build(
        &self,
        request: &InstalledBuildRequest,
    ) -> Result<InstalledCommandSuccess, CommandFailure> {
        if self.is_native_provider() {
            return self
                .build_with_provider(request, crate::CompilerProvider::Native, None)
                .map(installed_result);
        }
        match request.profile {
            InstalledProfile::I32 => crate::pipeline::execute_installed_scalar(self, request, None)
                .map(InstalledCommandSuccess::Scalar),
            InstalledProfile::ControlFlow => {
                crate::pipeline::execute_installed_control_flow(self, request, None)
                    .map(InstalledCommandSuccess::Scalar)
            }
            InstalledProfile::DataOwnership => {
                crate::ownership_pipeline::installed::build(self, request)
                    .map(InstalledCommandSuccess::Ownership)
            }
        }
    }

    /// Builds, invokes, and publishes a project using retained installation and project proofs.
    ///
    /// # Errors
    /// Rejects changed proofs, invalid invocations, and existing compiler or publication errors.
    pub fn run(
        &self,
        request: &InstalledBuildRequest,
        export: String,
        arguments: Vec<zryna_abi::ScalarValue>,
    ) -> Result<InstalledCommandSuccess, CommandFailure> {
        if self.is_native_provider() {
            return self
                .run_with_provider(
                    request,
                    export,
                    arguments,
                    crate::CompilerProvider::Native,
                    None,
                )
                .map(installed_result);
        }
        match request.profile {
            InstalledProfile::I32 => {
                crate::pipeline::execute_installed_scalar(self, request, Some((export, arguments)))
                    .map(InstalledCommandSuccess::Scalar)
            }
            InstalledProfile::ControlFlow => crate::pipeline::execute_installed_control_flow(
                self,
                request,
                Some((export, arguments)),
            )
            .map(InstalledCommandSuccess::Scalar),
            InstalledProfile::DataOwnership => {
                crate::ownership_pipeline::installed::run(self, request, export, arguments)
                    .map(InstalledCommandSuccess::Ownership)
            }
        }
    }
}

fn installed_result(result: crate::ProviderCommandSuccess) -> InstalledCommandSuccess {
    match result {
        crate::ProviderCommandSuccess::Scalar(value) => InstalledCommandSuccess::Scalar(value),
        crate::ProviderCommandSuccess::Ownership(value) => {
            InstalledCommandSuccess::Ownership(value)
        }
    }
}

impl InstalledCompiler {
    /// Reports the independently admitted installation's versioned frontend route.
    #[must_use]
    pub fn is_native_provider(&self) -> bool {
        !self.distribution.requires_bootstrap()
    }

    /// Builds from a native installation with explicit provider/optional target-host selection.
    ///
    /// # Errors
    /// Rejects legacy route substitution, absent bootstrap material and changed retained inputs.
    pub fn build_with_provider(
        &self,
        request: &InstalledBuildRequest,
        provider: crate::CompilerProvider,
        node_runtime: Option<std::path::PathBuf>,
    ) -> Result<crate::ProviderCommandSuccess, CommandFailure> {
        self.execute_provider(request, None, provider, node_runtime)
    }

    /// Executes an existing profile while retaining native installation and source authorities.
    ///
    /// # Errors
    /// Requires a separate Node host for JavaScript/WebAssembly execution and the existing SDK for native linking.
    pub fn run_with_provider(
        &self,
        request: &InstalledBuildRequest,
        export: String,
        arguments: Vec<zryna_abi::ScalarValue>,
        provider: crate::CompilerProvider,
        node_runtime: Option<std::path::PathBuf>,
    ) -> Result<crate::ProviderCommandSuccess, CommandFailure> {
        self.execute_provider(request, Some((export, arguments)), provider, node_runtime)
    }

    fn execute_provider(
        &self,
        request: &InstalledBuildRequest,
        run: Option<(String, Vec<zryna_abi::ScalarValue>)>,
        provider: crate::CompilerProvider,
        node_runtime: Option<std::path::PathBuf>,
    ) -> Result<crate::ProviderCommandSuccess, CommandFailure> {
        self.revalidate().map_err(crate::provider_api::preparation_failure)?;
        if !self.is_native_provider() {
            return Err(crate::provider_api::request_failure(
                "legacy installed packages retain their authenticated bootstrap route",
            ));
        }
        if provider != crate::CompilerProvider::Native {
            return Err(crate::provider_api::request_failure(
                "this native installation does not include bootstrap compatibility material",
            ));
        }
        let request = crate::ProviderBuildRequest {
            compiler_root: self.root.clone(),
            source: crate::CompilationSource::Project(request.project_root.clone()),
            entrypoint: request.entrypoint.clone(),
            artifact_stem: request.artifact_stem.clone(),
            targets: request.targets,
            profile: match request.profile {
                InstalledProfile::I32 => crate::CompilationProfile::I32,
                InstalledProfile::ControlFlow => crate::CompilationProfile::ControlFlow,
                InstalledProfile::DataOwnership => crate::CompilationProfile::DataOwnership,
            },
            provider,
            node_runtime,
        };
        crate::pipeline::providers::execute(&request, run, Some(self))
    }
}
