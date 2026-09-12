//! Shared sealed ownership lowering and backend preparation.

use super::{
    ArtifactOutputRoot, Checkpoint, CommandFailure, CommandFailureKind, DataOwnershipBuildRequest,
    DataOwnershipCandidateSuccess, Diagnostic, DispatchPhase, Invocation, NATIVE_TARGET,
    NativeProcessLimits, PreparedDataOwnershipArtifacts, ScalarValue,
    VerifiedOwnershipModuleClosure, discover_linux_native_toolchain, failure,
    prepare_data_ownership_executable,
};

pub(super) fn prepare_closure(
    request: &DataOwnershipBuildRequest,
    closure: VerifiedOwnershipModuleClosure,
    run: Option<(String, Vec<ScalarValue>)>,
    checkpoint: Checkpoint<'_>,
    validate: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<DataOwnershipCandidateSuccess, CommandFailure> {
    let program = closure
        .lower_data_ownership_v1()
        .map_err(|items| CommandFailure { kind: CommandFailureKind::Source, diagnostics: items })?;
    let invocation =
        run.as_ref().map(|(name, arguments)| Invocation::new(name.clone(), arguments.clone()));
    if let Some(invocation) = invocation.clone() {
        program.verified_ir().scalar_abi().prepare_invocation(invocation).map_err(|error| {
            failure(
                CommandFailureKind::Source,
                Diagnostic::error(
                    error.code(),
                    None,
                    "candidate invocation does not match the verified DataOwnershipV1 scalar ABI",
                    "use the exact entry export, arity, and typed scalar arguments",
                ),
            )
        })?;
    }
    validate()?;
    let artifacts = compile_selected(&program, request, invocation, checkpoint)?;
    let diagnostics = closure
        .syntax()
        .diagnostics()
        .iter()
        .filter(|item| item.severity() == zryna_diagnostics::Severity::Warning)
        .cloned()
        .collect();
    let (logical_export, arguments) =
        run.map_or((None, Vec::new()), |(name, args)| (Some(name), args));
    Ok(DataOwnershipCandidateSuccess {
        workspace_root: request.workspace_root.clone(),
        node_runtime: request.node_runtime.clone(),
        closure,
        program,
        artifacts,
        artifact_stem: request.artifact_stem.clone(),
        logical_export,
        arguments,
        diagnostics,
    })
}

fn compile_selected(
    program: &zryna_semantics::data_ownership_v1::VerifiedProgram,
    request: &DataOwnershipBuildRequest,
    invocation: Option<Invocation>,
    checkpoint: Checkpoint<'_>,
) -> Result<PreparedDataOwnershipArtifacts, CommandFailure> {
    let mut artifacts = PreparedDataOwnershipArtifacts::default();
    if request.targets.javascript() {
        checkpoint(DispatchPhase::JavaScript)?;
        artifacts.javascript = Some(
            zryna_backend_javascript::emit_data_ownership(
                program.verified_ir(),
                program.runtime_abi(),
            )
            .map_err(|item| failure(CommandFailureKind::Preparation, item))?,
        );
    }
    if request.targets.webassembly() {
        checkpoint(DispatchPhase::WebAssembly)?;
        artifacts.webassembly = Some(
            zryna_backend_webassembly::emit_data_ownership(
                program.verified_ir(),
                program.runtime_abi(),
            )
            .map_err(|item| failure(CommandFailureKind::Preparation, item))?,
        );
    }
    if request.targets.native() {
        checkpoint(DispatchPhase::Native)?;
        if let Some(invocation) = invocation {
            let output = ArtifactOutputRoot::prepare_for_workspace(&request.workspace_root)
                .map_err(|item| failure(CommandFailureKind::Preparation, item))?;
            let toolchain = discover_linux_native_toolchain(NativeProcessLimits::default())
                .map_err(|item| failure(CommandFailureKind::Preparation, item))?;
            artifacts.native_executable = Some(
                prepare_data_ownership_executable(
                    program,
                    invocation,
                    &output,
                    NATIVE_TARGET,
                    &toolchain,
                    NativeProcessLimits::default(),
                )
                .map_err(|items| CommandFailure {
                    kind: CommandFailureKind::Preparation,
                    diagnostics: items,
                })?,
            );
        } else {
            let target = crate::select_native_object_target(NATIVE_TARGET)
                .map_err(|item| failure(CommandFailureKind::Preparation, item))?;
            let mir = zryna_native_mir::data_ownership_v1::lower(
                program.verified_ir(),
                program.runtime_abi(),
            )
            .map_err(|items| CommandFailure {
                kind: CommandFailureKind::Preparation,
                diagnostics: items,
            })?;
            artifacts.native_object = Some(
                zryna_backend_native::data_ownership_v1::emit_object(&mir, target)
                    .map_err(|item| failure(CommandFailureKind::Preparation, item))?,
            );
        }
    }
    Ok(artifacts)
}
