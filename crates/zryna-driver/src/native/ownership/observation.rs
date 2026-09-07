use super::super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn run(
    executable: &PreparedNativeExecutable,
    output_root: &ArtifactOutputRoot,
    limits: NativeProcessLimits,
    fault: Option<u32>,
) -> Result<Vec<u8>, NativeRunError> {
    ensure_linux_x86_64_host().map_err(native_run_error)?;
    let stage = NativeStage::create(output_root, "run").map_err(native_run_error)?;
    let operation = (|| {
        stage.write_input(&stage.executable, executable.bytes())?;
        let executable_path = stage.capability_file_path("invocation.elf")?;
        let directory_path = stage.capability_directory_path();
        prepare_executable_mode(&executable_path)?;
        stage.revalidate()?;
        run_bounded_process(
            &executable_path,
            &fault.map(|value| vec![OsString::from(value.to_string())]).unwrap_or_default(),
            &directory_path,
            limits.run_timeout(),
            12 + 4096 * 4 + 1,
            limits.run_stderr_bytes(),
            ProcessPhase::Run,
            Some(&directory_path),
        )
    })();
    let cleanup = stage.cleanup();
    if let Some(diagnostic) = cleanup.into_iter().next() {
        return Err(native_run_error(diagnostic));
    }
    let output = operation.map_err(native_run_error)?;
    if !output.status.success()
        || !output.stderr.is_empty()
        || output.stdout.len() < 12
        || output.stdout.len() > 12 + 4096 * 4
    {
        return Err(native_run_error(native_error(
            "ZRYNA-N4022",
            "candidate invocation returned an invalid observation frame",
            "report the exact authenticated candidate invocation",
        )));
    }
    Ok(output.stdout)
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub(crate) fn run(
    _executable: &PreparedNativeExecutable,
    _output_root: &ArtifactOutputRoot,
    _limits: NativeProcessLimits,
    _fault: Option<u32>,
) -> Result<Vec<u8>, NativeRunError> {
    Err(native_run_error(native_error(
        "ZRYNA-N4002",
        "native invocation requires Linux x86-64",
        "run on the supported host",
    )))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn run_published(
    executable: &PublishedNativeExecutableArtifact,
    limits: NativeProcessLimits,
) -> Result<zryna_abi::ScalarOutcome, NativeRunError> {
    if executable.data_ownership_identity().is_none() {
        return run_prepared_native_invocation(
            &executable.prepared,
            &executable.output_root,
            limits,
        );
    }
    let frame = run(&executable.prepared, &executable.output_root, limits, None)?;
    crate::ownership_commands::observation::decode(
        &frame,
        executable.prepared.result_type(),
        crate::OwnershipTarget::Native,
    )
    .map(|result| result.outcome())
    .map_err(|failure| native_run_error(failure.diagnostics()[0].clone()))
}
