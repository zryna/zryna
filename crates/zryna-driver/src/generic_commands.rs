//! Public generic commands retain original source/provider handles through terminal commit.
use crate::{
    ArtifactOutputRoot, CommandFailure, CommandFailureKind, generic_closure, generic_pipeline,
    generic_run, pipeline::BuildInputs, provider_api::ProviderSession, source_scope,
    source_session::ModuleSourceRoot,
};
use zryna_abi::ScalarValue;
use zryna_diagnostics::Diagnostic;
use zryna_source::NormalizedSourcePath;

pub(crate) fn execute(
    inputs: &BuildInputs,
    entrypoint: NormalizedSourcePath,
    source: &impl ModuleSourceRoot,
    session: &ProviderSession<'_>,
    run: Option<(String, Vec<ScalarValue>)>,
    validate: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<generic_pipeline::PublishedGenericBundle, CommandFailure> {
    let output = ArtifactOutputRoot::prepare_for_workspace(&inputs.workspace_root)
        .map_err(crate::provider_api::preparation_failure)?;
    let command = if run.is_some() { "run" } else { "build" };
    crate::pipeline::ensure_absent(
        &output.path().join(format!("{}.{command}", inputs.artifact_stem)),
    )?;
    let frontend = session.frontend_v5()?;
    let hosts = run
        .as_ref()
        .map(|_| generic_run::GenericRunHosts::prepare(inputs.targets, session.node()))
        .transpose()?;
    let (export, arguments) = match run {
        Some((export, arguments)) => (Some(export), Some(arguments)),
        None => (None, None),
    };
    generic_closure::with_closure_terminal(
        source,
        entrypoint,
        &*frontend,
        validate,
        |closure, checkpoint| {
            source_scope::with_execution_for_export(
                closure,
                export.as_deref(),
                checkpoint,
                |program, receipt, checkpoint| {
                    let mut prepared =
                        generic_pipeline::prepare(program, receipt, inputs.targets, checkpoint)?;
                    prepared.retain_advisories(&advisories(closure)?);
                    let completed = match (export.as_ref(), arguments, hosts.as_ref()) {
                        (Some(export), Some(arguments), Some(hosts)) => {
                            // Establish the source-bounded name before cloning a raw request string.
                            if program.scalar_abi().export(export).is_none() {
                                return Err(CommandFailure {
                                    kind: CommandFailureKind::Request,
                                    diagnostics: vec![Diagnostic::error(
                                        "ZRYNA-B2101",
                                        None,
                                        "generic invocation selects no scalar source export",
                                        "select an exact nongeneric scalar export from the entry module",
                                    )],
                                });
                            }
                            Some(generic_run::execute(
                                program,
                                receipt,
                                inputs.targets,
                                hosts,
                                &output,
                                export.clone(),
                                arguments,
                                checkpoint,
                            )?)
                        }
                        (None, None, None) => None,
                        _ => return Err(invariant()),
                    };
                    Ok((prepared, completed))
                },
            )
        },
        |(prepared, completed), checkpoint| match completed {
            Some(completed) => generic_pipeline::publish_run(
                prepared,
                completed,
                &output,
                &inputs.artifact_stem,
                checkpoint,
            ),
            None => generic_pipeline::publish(prepared, &output, &inputs.artifact_stem, checkpoint),
        },
    )
}

fn invariant() -> CommandFailure {
    CommandFailure {
        kind: CommandFailureKind::Preparation,
        diagnostics: vec![Diagnostic::error(
            "ZRYNA-C3413",
            None,
            "generic command lost its exact prepared request boundary",
            "retain the same source request, selected hosts and complete invocation",
        )],
    }
}

fn advisories(
    closure: &generic_closure::VerifiedGenericClosure,
) -> Result<Vec<Diagnostic>, CommandFailure> {
    use zryna_syntax::v4::RawDiagnosticLocation;
    let mut diagnostics = Vec::new();
    diagnostics.try_reserve_exact(closure.syntax().advisories().len()).map_err(|_| invariant())?;
    for item in closure.syntax().advisories() {
        if item.severity != zryna_diagnostics::Severity::Warning {
            return Err(invariant());
        }
        let diagnostic = match item.location {
            RawDiagnosticLocation::Global => Diagnostic::warning(
                item.code.clone(),
                None,
                item.message.clone(),
                item.guidance.clone(),
            ),
            RawDiagnosticLocation::Source { span } => Diagnostic::warning_at(
                item.code.clone(),
                closure.sources().verify_span(span).map_err(|_| invariant())?,
                item.message.clone(),
                item.guidance.clone(),
            ),
        };
        diagnostics.push(diagnostic);
    }
    Ok(diagnostics)
}
