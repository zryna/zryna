//! Native/default and explicit compatibility dispatch through the original owning phases.

use std::{cell::RefCell, time::Instant};

use zryna_abi::ScalarValue;
use zryna_diagnostics::Diagnostic;
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};

use super::{BuildInputs, CommandFailure, CommandFailureKind, CommandKind, RunInvocation};
use crate::provider_api::{
    CompilationProfile, CompilationSource, ProviderBuildRequest, ProviderCommandSuccess,
    ProviderSession, preparation_failure, request_failure,
};
use crate::source_session::{ModuleSourceRoot, ModuleSourceSession};

pub(crate) fn execute(
    request: &ProviderBuildRequest,
    run: Option<(String, Vec<ScalarValue>)>,
    installation: Option<&crate::distribution::InstalledCompiler>,
) -> Result<ProviderCommandSuccess, CommandFailure> {
    if installation.is_none()
        && matches!(request.source, CompilationSource::Project(_))
        && request.profile != CompilationProfile::I32
    {
        return Err(CommandFailure {
            kind: CommandFailureKind::Request,
            diagnostics: vec![Diagnostic::error(
                "ZRYNA-C2001",
                None,
                "standalone projects currently require the manifest-declared i32-v1 profile",
                "omit --profile when using --project-root",
            )],
        });
    }
    if !request.compiler_root.is_absolute() || !request.source.root().is_absolute() {
        return Err(request_failure("compiler and source roots must be absolute"));
    }
    if let Some(path) = &request.node_runtime {
        if !path.is_absolute() {
            return Err(request_failure(
                "an explicit Node target host must be an absolute direct path",
            ));
        }
    }
    match installation {
        Some(installation) => installation.revalidate().map_err(preparation_failure)?,
        None => super::validate_architecture(&request.compiler_root)?,
    }
    let inputs = BuildInputs {
        workspace_root: request.source.root().to_path_buf(),
        entrypoint: request.entrypoint.clone(),
        artifact_stem: request.artifact_stem.clone(),
        targets: request.targets,
    };
    let command = if run.is_some() { CommandKind::Run } else { CommandKind::Build };
    super::validate_build_inputs(&inputs, command)?;
    if request.profile != CompilationProfile::I32 && request.targets.component() {
        return Err(super::unsupported_component_request(
            "component emission currently accepts only the default scalar profile",
        ));
    }
    let session = ProviderSession::prepare(request, run.is_some())?;
    let outer_checkpoint = || {
        session.revalidate()?;
        if let Some(installation) = installation {
            installation.revalidate().map_err(preparation_failure)?;
        }
        Ok(())
    };
    match &request.source {
        CompilationSource::Workspace(root) => {
            let sources = crate::WorkspaceSourceRoot::capture(root).map_err(source_failure)?;
            dispatch(request, &inputs, run, &sources, &session, &outer_checkpoint)
        }
        CompilationSource::Project(_) => {
            let project = crate::project::ProjectAdmission::discover_provider(
                &request.compiler_root,
                &inputs,
                request.profile.package_profile(),
                installation.is_some(),
            )?;
            dispatch(request, &inputs, run, &project, &session, &|| {
                outer_checkpoint()?;
                project.revalidate()
            })
        }
    }
}

fn dispatch<Source: ModuleSourceRoot>(
    request: &ProviderBuildRequest,
    inputs: &BuildInputs,
    run: Option<(String, Vec<ScalarValue>)>,
    source: &Source,
    session: &ProviderSession<'_>,
    validate: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<ProviderCommandSuccess, CommandFailure> {
    let entrypoint = NormalizedSourcePath::new(request.entrypoint.clone())
        .map_err(|error| source_failure(Diagnostic::from_source_error(&error)))?;
    validate()?;
    if request.profile == CompilationProfile::DataOwnership {
        let frontend = session.frontend_v4()?;
        let closure = crate::ownership_closure::discover_with_clock(
            source,
            entrypoint,
            &*frontend,
            Instant::now,
        )
        .map_err(|error| super::module_closure_failure(&error))?;
        validate()?;
        let prepared = crate::ownership_pipeline::preparation::prepare_closure(
            inputs,
            request.node_runtime.clone(),
            closure,
            run,
            &|_| validate(),
            validate,
        )?;
        return crate::ownership_commands::execute_with_provider(
            &prepared,
            session.node(),
            validate,
        )
        .map(ProviderCommandSuccess::Ownership);
    }
    let command = if run.is_some() { CommandKind::Run } else { CommandKind::Build };
    let invocation =
        run.map(|(logical_export, arguments)| RunInvocation { logical_export, arguments });
    let output = crate::ArtifactOutputRoot::prepare_for_workspace(&inputs.workspace_root)
        .map_err(preparation_failure)?;
    let final_bundle = output.path().join(format!("{}.{}", inputs.artifact_stem, command.suffix()));
    super::ensure_absent(&final_bundle)?;
    match request.profile {
        CompilationProfile::I32 => {
            let source_session = RefCell::new(source.begin().map_err(source_failure)?);
            let stable =
                source_session.borrow_mut().read_source(&entrypoint).map_err(source_failure)?;
            let sources = SourceMap::build(vec![SourceFileInput {
                path: request.entrypoint.clone(),
                text: stable.text.clone(),
            }])
            .map_err(|error| source_failure(Diagnostic::from_source_error(&error)))?;
            source_session
                .borrow_mut()
                .validate_provider_batch(&sources)
                .map_err(source_failure)?;
            let frontend = session.frontend_v2()?;
            let compiled = super::analyze(&*frontend, &sources)?;
            validate()?;
            source_session
                .borrow_mut()
                .validate_provider_batch(&sources)
                .map_err(source_failure)?;
            super::scalar::finish(
                inputs,
                invocation.as_ref(),
                session.node(),
                &output,
                &final_bundle,
                &stable.text,
                &compiled,
                &|| {
                    validate()?;
                    source_session
                        .borrow_mut()
                        .validate_provider_batch(&sources)
                        .map_err(source_failure)?;
                    source_session.borrow_mut().revalidate_all().map_err(source_failure)
                },
            )
            .map(ProviderCommandSuccess::Scalar)
        }
        CompilationProfile::ControlFlow => {
            let frontend = session.frontend_v3()?;
            let closure = crate::module_closure::discover_module_closure_with_clock(
                source,
                entrypoint,
                &*frontend,
                Instant::now,
            )
            .map_err(|error| super::module_closure_failure(&error))?;
            validate()?;
            super::control_flow::finish(
                inputs,
                invocation.as_ref(),
                session.node(),
                &output,
                &final_bundle,
                &closure,
                &|_| validate(),
            )
            .map(ProviderCommandSuccess::Scalar)
        }
        CompilationProfile::DataOwnership => unreachable!("handled before scalar publication"),
    }
}

fn source_failure(diagnostic: Diagnostic) -> CommandFailure {
    CommandFailure { kind: CommandFailureKind::Source, diagnostics: vec![diagnostic] }
}
