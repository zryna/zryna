//! Bounded process adapter for the private allocation-runtime C fixture.

use std::{ffi::OsString, path::Path};

use zryna_diagnostics::Diagnostic;

use super::super::{
    MAX_NATIVE_RUN_STDERR_BYTES, NativeProcessLimits, ProcessPhase, native_error,
    run_bounded_process,
};

pub(crate) fn compile_and_run(
    source: &Path,
    root: &Path,
    expected: &[u8],
) -> Result<(), Diagnostic> {
    let limits = NativeProcessLimits::default();
    let executable = root.join("allocation-observation");
    let arguments = [
        OsString::from("-std=c11"),
        OsString::from("-pedantic"),
        OsString::from("-Wall"),
        OsString::from("-Wextra"),
        OsString::from("-Werror"),
        OsString::from("-O2"),
        OsString::from("-fno-common"),
        source.as_os_str().to_owned(),
        OsString::from("-o"),
        executable.as_os_str().to_owned(),
    ];
    let compiled = run_bounded_process(
        Path::new("/usr/bin/gcc"),
        &arguments,
        root,
        limits.link_timeout(),
        limits.tool_output_bytes(),
        limits.tool_output_bytes(),
        ProcessPhase::Link,
        Some(root),
    )?;
    if !compiled.status.success() || !compiled.stdout.is_empty() || !compiled.stderr.is_empty() {
        return Err(native_error(
            "ZRYNA-N4017",
            "the validated GNU toolchain rejected the private allocation fixture",
            "verify the documented system toolchain and report the fixture diagnostics",
        ));
    }

    let output = run_bounded_process(
        &executable,
        &[],
        root,
        limits.run_timeout(),
        expected.len(),
        MAX_NATIVE_RUN_STDERR_BYTES,
        ProcessPhase::Run,
        Some(root),
    )?;
    if !output.status.success() {
        return Err(native_error(
            "ZRYNA-N4021",
            "the private allocation fixture exited abnormally",
            "report the smallest reproducible allocation fixture failure",
        ));
    }
    if output.stdout != expected || !output.stderr.is_empty() {
        return Err(native_error(
            "ZRYNA-N4022",
            "the private allocation fixture returned an invalid result frame",
            "report the smallest reproducible allocation fixture output",
        ));
    }
    Ok(())
}
