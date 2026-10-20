//! Provider request admission and profile-owned rendering.

use super::{
    CliProfile, CliTarget, CommandKind, CompileOptions, TargetSelection, absolute_workspace_path,
    command_h1, ownership, profile, render_failure, render_success,
};
use std::{path::PathBuf, process::ExitCode};
use zryna_diagnostics::Diagnostic;

pub(super) fn render_provider_result(
    result: Result<zryna_driver::ProviderCommandSuccess, zryna_driver::CommandFailure>,
    command: CommandKind,
    json: bool,
) -> ExitCode {
    match result {
        Ok(zryna_driver::ProviderCommandSuccess::Scalar(success)) => render_success(&success, json),
        Ok(zryna_driver::ProviderCommandSuccess::Ownership(success)) => {
            ownership::render(Ok(success), command, json)
        }
        Err(failure) => render_failure(command, json, &failure),
    }
}

pub(super) fn provider_request(
    options: CompileOptions,
) -> Result<zryna_driver::ProviderBuildRequest, Diagnostic> {
    let compiler_root =
        absolute_workspace_path(&options.root.unwrap_or_else(|| PathBuf::from(".")))?;
    let source = match options.project_root {
        Some(root) => zryna_driver::CompilationSource::Project(absolute_workspace_path(&root)?),
        None => zryna_driver::CompilationSource::Workspace(compiler_root.clone()),
    };
    let targets = if options.profile == Some(CliProfile::BrowserComponentV1) {
        if options.target != CliTarget::Component {
            return Err(Diagnostic::error(
                "ZRYNA-C1012",
                None,
                "browser-component-v1 requires the component build target",
                "select --target component",
            ));
        }
        TargetSelection::BrowserComponent
    } else {
        options.target.ordinary()?
    };
    let profile = match options.profile {
        None | Some(CliProfile::BrowserComponentV1) => zryna_driver::CompilationProfile::I32,
        Some(CliProfile::ControlFlowV1) => zryna_driver::CompilationProfile::ControlFlow,
        Some(CliProfile::DataOwnershipV1) => zryna_driver::CompilationProfile::DataOwnership,
        Some(CliProfile::CommandH1V1) => return Err(command_h1::configuration_error()),
    };
    Ok(zryna_driver::ProviderBuildRequest {
        compiler_root,
        source,
        entrypoint: options.entrypoint.clone(),
        artifact_stem: options.name.unwrap_or_else(|| profile::default_stem(&options.entrypoint)),
        targets,
        profile,
        provider: options.provider.map(Into::into).unwrap_or_default(),
        node_runtime: options.node,
    })
}
