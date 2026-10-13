//! Dedicated instrumented C translation units; original Rust/Zryna objects stay unchanged.
use super::*;
const FLAGS: [&str; 3] =
    ["-fsanitize=address,undefined", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer"];
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
    args.extend(FLAGS.map(Into::into));
    args.extend([
        stage.capability_file_path("invocation.c")?.into_os_string(),
        "-o".into(),
        stage.capability_file_path("runtime.o")?.into_os_string(),
    ]);
    let driver = session.tools.native.driver.clone();
    process::invoke(session, stage, name, &driver, args, false)?;
    process::successful(session)?;
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
    args.extend(FLAGS.map(Into::into));
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
    process::invoke(session, &stages[0], "link", &driver, args, false)?;
    // Retain the actual product before success/audit comparison; it may diagnose an error.
    if let Ok(bytes) = material::read(
        &stages[0].directory.join("invocation.elf"),
        native::MAX_NATIVE_EXECUTABLE_BYTES,
    ) {
        session.artifacts.push(("final.elf".into(), bytes));
    }
    process::successful(session)
}
