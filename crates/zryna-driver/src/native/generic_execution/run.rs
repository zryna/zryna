//! Execute only private copies of the captured GNU loader/libc and exact prepared image.

use super::{PreparedGenericExecutable, provider_stage::ProviderStage, wire};
use crate::native::{
    ArtifactOutputRoot, MAX_NATIVE_EXECUTABLE_BYTES, NativeProcessLimits, NativeStage,
    ProcessPhase, ToolFileIdentity, prepare_executable_mode, run_bounded_process,
};
use sha2::{Digest as _, Sha256};
use std::ffi::OsString;
use zryna_abi::ScalarOutcome;
use zryna_diagnostics::Diagnostic;

pub(crate) fn run(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    output: &ArtifactOutputRoot,
    limits: NativeProcessLimits,
) -> Result<ScalarOutcome, Vec<Diagnostic>> {
    prepared.dependencies.revalidate().map_err(|error| vec![error])?;
    let stage = NativeStage::create(output, "generic-run").map_err(|error| vec![error])?;
    let result = (|| {
        stage.write_input(&stage.executable, prepared.bytes()).map_err(|error| vec![error])?;
        let executable =
            stage.capability_file_path("invocation.elf").map_err(|error| vec![error])?;
        prepare_executable_mode(&executable).map_err(|error| vec![error])?;
        let identity = image(prepared, &stage, None).map_err(|error| vec![error])?;
        let providers = ProviderStage::create(&stage, &prepared.dependencies)?;
        let operation = execute(prepared, &stage, &providers, &identity, limits);
        let cleanup = providers.cleanup();
        match operation {
            Ok(outcome) if cleanup.is_empty() => Ok(outcome),
            Ok(_) => Err(cleanup),
            Err(mut errors) => {
                errors.extend(cleanup);
                Err(errors)
            }
        }
    })();
    let cleanup = stage.cleanup();
    match result {
        Ok(outcome) if cleanup.is_empty() => Ok(outcome),
        Ok(_) => Err(cleanup),
        Err(mut errors) => {
            errors.extend(cleanup);
            Err(errors)
        }
    }
}

fn image(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    stage: &NativeStage,
    expected: Option<&ToolFileIdentity>,
) -> Result<ToolFileIdentity, Diagnostic> {
    stage.revalidate()?;
    let path = stage.capability_file_path("invocation.elf")?;
    let (identity, bytes) = super::stable::read(&path, MAX_NATIVE_EXECUTABLE_BYTES)?;
    if expected.is_some_and(|expected| expected != &identity)
        || <[u8; 32]>::from(Sha256::digest(&bytes))
            != *prepared.identity().image().executable_sha256()
    {
        return Err(super::staging_error());
    }
    Ok(identity)
}

fn execute(
    prepared: &PreparedGenericExecutable<'_, '_, '_>,
    stage: &NativeStage,
    providers: &ProviderStage<'_>,
    identity: &ToolFileIdentity,
    limits: NativeProcessLimits,
) -> Result<ScalarOutcome, Vec<Diagnostic>> {
    let validate = || {
        prepared.dependencies.revalidate()?;
        providers.revalidate(&prepared.dependencies)?;
        image(prepared, stage, Some(identity)).map(|_| ())
    };
    validate().map_err(|error| vec![error])?;
    let directory = stage.capability_directory_path();
    let executable = stage.capability_file_path("invocation.elf").map_err(|error| vec![error])?;
    let args = [
        OsString::from("--inhibit-cache"),
        OsString::from("--library-path"),
        providers.directory().into_os_string(),
        executable.into_os_string(),
    ];
    let result = run_bounded_process(
        &providers.loader(),
        &args,
        &directory,
        limits.run_timeout(),
        wire::BYTES,
        limits.run_stderr_bytes(),
        ProcessPhase::Run,
        Some(&directory),
    );
    let validation = validate();
    let mut errors = Vec::new();
    if let Err(error) = validation {
        errors.push(error);
    }
    let actual = match result {
        Ok(actual) => actual,
        Err(error) => {
            errors.insert(0, error);
            return Err(errors);
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    if !actual.status.success() || !actual.stderr.is_empty() {
        return Err(vec![host_process_error()]);
    }
    wire::decode(&actual.stdout, prepared.identity().result_type()).map_err(|error| vec![error])
}

fn host_process_error() -> Diagnostic {
    super::super::native_error(
        "ZRYNA-C4169",
        "the generic native host process exited unsuccessfully or wrote unexpected stderr",
        "retain the captured private GNU loader and exact invocation image; host exit or signal failures are not source language traps",
    )
}
