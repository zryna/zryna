//! Ordinary C processes and target comparison; Rust observations never become GCC invocations.
use super::*;
use std::{cell::Cell, ffi::OsString, path::Path, time::Instant};
thread_local! { static ATTEMPTS:Cell<usize>=const{Cell::new(0)}; }
pub(super) fn attempts() -> usize {
    ATTEMPTS.with(Cell::get)
}

#[derive(Debug)]
pub(super) struct Record {
    pub(super) name: &'static str,
    pub(super) program: PathBuf,
    pub(super) arguments: Vec<OsString>,
    pub(super) elapsed_ms: u128,
    pub(super) directory: PathBuf,
    pub(super) output: Option<native::BoundedProcessOutput>,
    pub(super) diagnostic: Option<Diagnostic>,
}
// Clone only an existing actual observation; no constructor from serialized success.
impl Clone for Record {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            program: self.program.clone(),
            arguments: self.arguments.clone(),
            elapsed_ms: self.elapsed_ms,
            directory: self.directory.clone(),
            output: self.output.as_ref().map(|o| native::BoundedProcessOutput {
                status: o.status,
                stdout: o.stdout.clone(),
                stderr: o.stderr.clone(),
            }),
            diagnostic: self.diagnostic.clone(),
        }
    }
}
pub(super) fn invoke(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    name: &'static str,
    program: &Path,
    args: Vec<OsString>,
    run: bool,
) -> Result<(), Diagnostic> {
    session.phase = name;
    session.tools.check()?;
    stage.revalidate()?;
    let directory = stage.capability_directory_path();
    if let Some(rust) = session.rust_tools {
        rust.check()?;
    }
    if !run
        && program != session.tools.native.driver
        && program != session.tools.objdump
        && session.rust_tools.is_none_or(|rust| program != rust.rustc)
    {
        return Err(rejected());
    }
    if run && program != stage.capability_file_path("invocation.elf")? {
        return Err(rejected());
    }
    ATTEMPTS.with(|value| value.set(value.get() + 1));
    let started = Instant::now();
    let limits = native::NativeProcessLimits::default();
    let result = native::run_bounded_process(
        program,
        &args,
        &directory,
        if run { limits.run_timeout() } else { limits.link_timeout() },
        if run { 4096 } else { limits.tool_output_bytes() },
        if run { limits.run_stderr_bytes() } else { limits.tool_output_bytes() },
        if run { native::ProcessPhase::Run } else { native::ProcessPhase::Link },
        Some(&directory),
    );
    let (output, diagnostic) = match result {
        Ok(value) => (Some(value), None),
        Err(value) => (None, Some(value)),
    };
    session.records.push(Record {
        name,
        program: program.to_owned(),
        arguments: args,
        elapsed_ms: started.elapsed().as_millis(),
        directory,
        output,
        diagnostic,
    });
    stage.revalidate()?;
    session.tools.check()?;
    if let Some(rust) = session.rust_tools {
        rust.check()?;
    }
    let record = session.records.last().ok_or_else(rejected)?;
    if let Some(error) = &record.diagnostic {
        return Err(error.clone());
    }
    Ok(())
}
pub(super) fn successful(session: &Session<'_>) -> Result<(), Diagnostic> {
    let output = session.records.last().and_then(|r| r.output.as_ref()).ok_or_else(rejected)?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(rejected());
    }
    Ok(())
}
pub(super) fn compile(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    name: &'static str,
    source: &[u8],
) -> Result<(), Diagnostic> {
    if source.len() > native::MAX_NATIVE_HARNESS_BYTES {
        return Err(rejected());
    }
    stage.write_input(&stage.harness, source)?;
    let mut args: Vec<OsString> = [
        "-std=c11",
        "-O0",
        "-g0",
        "-m64",
        "-fno-pie",
        "-fno-pic",
        "-fcf-protection=none",
        "-fno-stack-protector",
        "-fno-unwind-tables",
        "-fno-asynchronous-unwind-tables",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-c",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.extend([
        stage.capability_file_path("invocation.c")?.into_os_string(),
        "-o".into(),
        stage.capability_file_path("runtime.o")?.into_os_string(),
    ]);
    let driver = session.tools.native.driver.clone();
    invoke(session, stage, name, &driver, args, false)?;
    successful(session)?;
    let bytes = material::read(
        &stage.directory.join("runtime.o"),
        zryna_backend_native::MAX_NATIVE_OBJECT_BYTES,
    )?;
    if bytes.is_empty() {
        return Err(rejected());
    }
    session.artifacts.push((format!("{name}.o"), bytes));
    Ok(())
}
pub(super) fn link(
    session: &mut Session<'_>,
    stages: &[native::NativeStage],
    mode: Mode,
) -> Result<(), Diagnostic> {
    let mut args: Vec<OsString> = [
        "-m64",
        "-no-pie",
        "-Wl,--build-id=none",
        "-Wl,--fatal-warnings",
        "-Wl,--no-undefined",
        "-Wl,-z,noexecstack,-z,relro,-z,now",
        "-Wl,-t,-t",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    match mode {
        Mode::Scalar => args.push("-Wl,--wrap=add".into()),
        Mode::Bytes { .. } => args.extend(
            ["-Wl,--wrap=sum_bytes", "-Wl,--wrap=malloc", "-Wl,--wrap=free"].map(Into::into),
        ),
        Mode::Reverse => {}
    }
    args.extend([
        stages[1].capability_file_path("runtime.o")?.into_os_string(),
        stages[0].capability_file_path("program.o")?.into_os_string(),
    ]);
    if mode != Mode::Reverse {
        args.push(stages[0].capability_file_path("runtime.o")?.into_os_string());
    }
    if matches!(mode, Mode::Bytes { .. }) {
        args.push(stages[2].capability_file_path("runtime.o")?.into_os_string());
    }
    args.extend(["-o".into(), stages[0].capability_file_path("invocation.elf")?.into_os_string()]);
    let driver = session.tools.native.driver.clone();
    invoke(session, &stages[0], "link", &driver, args, false)?;
    // Retain the actual product before success/audit comparison; it may diagnose an error.
    if let Ok(bytes) = material::read(
        &stages[0].directory.join("invocation.elf"),
        native::MAX_NATIVE_EXECUTABLE_BYTES,
    ) {
        session.artifacts.push(("final.elf".into(), bytes));
    }
    successful(session)
}
pub(super) fn run(
    session: &mut Session<'_>,
    stage: &native::NativeStage,
    mode: Mode,
) -> Result<(), Diagnostic> {
    let executable = stage.capability_file_path("invocation.elf")?;
    let symbol = if mode == Mode::Reverse { "zryna_c_v0_e_add" } else { "zryna_c_v0_i_dispatch" };
    session.phase = "final-elf-audit";
    let (_, audited) = native::audit_staged_executable(&executable, symbol)?;
    let expected =
        &session.artifacts.iter().find(|(name, _)| name == "final.elf").ok_or_else(rejected)?.1;
    if audited.as_ref() != expected.as_slice() {
        return Err(rejected());
    }
    if mode != Mode::Reverse && !session.targets_checked {
        return Err(rejected());
    }
    native::prepare_executable_mode(&executable)?;
    if material::read(&stage.directory.join("invocation.elf"), native::MAX_NATIVE_EXECUTABLE_BYTES)?
        != *expected
    {
        return Err(rejected());
    }
    invoke(session, stage, "run", &executable, Vec::new(), true)?;
    let current = material::read(
        &stage.directory.join("invocation.elf"),
        native::MAX_NATIVE_EXECUTABLE_BYTES,
    )?;
    let expected =
        &session.artifacts.iter().find(|(name, _)| name == "final.elf").ok_or_else(rejected)?.1;
    if current != *expected {
        session.artifacts.push(("final-after-run.elf".into(), current));
        return Err(rejected());
    }
    Ok(())
}
