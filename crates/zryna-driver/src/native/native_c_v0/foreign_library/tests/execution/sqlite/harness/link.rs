//! Fixed bounded link arguments; no library search paths or extra runtime grants.
use super::*;

pub(super) fn link_arguments(
    stage: &NativeStage,
    runtime: PathBuf,
    provider: PathBuf,
    sanitized: bool,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-std=c11",
        "-O0",
        "-g0",
        "-fno-ident",
        "-fno-pie",
        "-no-pie",
        "-Wl,--build-id=none",
        "-Wl,--fatal-warnings",
        "-Wl,--no-undefined",
        "-Wl,-z,noexecstack,-z,relro,-z,now",
        "-Wl,--wrap=malloc",
        "-Wl,--wrap=free",
        "-Wl,--wrap=sqlite3_malloc",
        "-Wl,--wrap=sqlite3_free",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    if sanitized {
        args.extend(SANITIZERS.map(Into::into));
    }
    args.extend([
        "-o".into(),
        stage.capability_file_path("invocation.elf").expect("executable").into_os_string(),
    ]);
    for name in ["invocation.c", "program.o", "runtime.o"] {
        args.push(stage.capability_file_path(name).expect("fixed link input").into_os_string());
    }
    args.extend([runtime.into_os_string(), provider.into_os_string(), "-lm".into()]);
    args
}
