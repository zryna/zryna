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
