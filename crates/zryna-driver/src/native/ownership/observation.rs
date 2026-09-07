use super::super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn run(
    executable: &PreparedNativeExecutable,
    output_root: &ArtifactOutputRoot,
    limits: NativeProcessLimits,
) -> Result<[u8; 8], NativeRunError> {
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
            &[],
            &directory_path,
            limits.run_timeout(),
            9,
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
    if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() != 8 {
        return Err(native_run_error(native_error(
            "ZRYNA-N4022",
            "candidate invocation returned an invalid observation frame",
            "report the exact authenticated candidate invocation",
        )));
    }
    output.stdout.try_into().map_err(|_| {
        native_run_error(native_error(
            "ZRYNA-N4022",
            "invalid candidate observation length",
            "report the invocation",
        ))
    })
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub(crate) fn run(
    _executable: &PreparedNativeExecutable,
    _output_root: &ArtifactOutputRoot,
    _limits: NativeProcessLimits,
) -> Result<[u8; 8], NativeRunError> {
    Err(native_run_error(native_error(
        "ZRYNA-N4002",
        "native invocation requires Linux x86-64",
        "run on the supported host",
    )))
}
