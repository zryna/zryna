//! Only the captured GNU driver selects the two fixed runtime provider roles.

use super::{
    CapturedFile, Dependencies, DependencyIdentity, INTERPRETER, LIBC, LOADER, ToolSeal, rejected,
};
use crate::native::{
    LinuxX8664LinkToolchain, NativeProcessLimits, NativeStage, ProcessPhase, run_bounded_process,
};
use std::{ffi::OsString, path::PathBuf};
use zryna_diagnostics::Diagnostic;

pub(super) fn issue(
    stage: &NativeStage,
    tools: &LinuxX8664LinkToolchain,
    limits: NativeProcessLimits,
) -> Result<Dependencies, Diagnostic> {
    let tools = ToolSeal::capture(tools)?;
    let loader = CapturedFile::capture(probe(stage, &tools, LOADER, limits)?)?;
    let libc = CapturedFile::capture(probe(stage, &tools, LIBC, limits)?)?;
    loader.revalidate_alias(std::path::Path::new(INTERPRETER))?;
    let identity = DependencyIdentity {
        driver_sha256: tools.driver_sha256,
        linker_sha256: tools.linker_sha256,
        loader_sha256: loader.sha256,
        libc_sha256: libc.sha256,
        interpreter: INTERPRETER,
        gcc_version: tools.tools.gcc_version().into(),
        linker_version: tools.tools.linker_version().into(),
    };
    let result = Dependencies { tools, loader, libc, identity };
    result.revalidate()?;
    Ok(result)
}

fn probe(
    stage: &NativeStage,
    tools: &ToolSeal,
    role: &str,
    limits: NativeProcessLimits,
) -> Result<PathBuf, Diagnostic> {
    stage.revalidate()?;
    tools.revalidate()?;
    let directory = stage.capability_directory_path();
    let output = run_bounded_process(
        &tools.tools.driver,
        &[OsString::from(format!("-print-file-name={role}"))],
        &directory,
        limits.probe_timeout(),
        limits.tool_output_bytes(),
        limits.tool_output_bytes(),
        ProcessPhase::Probe,
        Some(&directory),
    )?;
    tools.revalidate()?;
    stage.revalidate()?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(rejected());
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| rejected())?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() || text.len() > 4096 || text.contains(['\n', '\r', '\0']) {
        return Err(rejected());
    }
    let path = PathBuf::from(text);
    if !path.is_absolute() {
        return Err(rejected());
    }
    Ok(path)
}
