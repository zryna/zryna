//! Real canonical C17 compilation using the unchanged same-seal metadata.
use super::process;
use std::{path::Path, process::Command};
const SYMBOLS: [&str; 3] = ["vec_allocate", "vec_reserve", "vec_release_storage"];
fn success(command: Command, seconds: u64) -> std::process::Output {
    let output = process::run(command, seconds);
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}
pub(super) fn compile(directory: &Path) {
    let mut machine = Command::new("/usr/bin/gcc");
    machine.arg("-dumpmachine");
    assert_eq!(success(machine, 10).stdout, b"x86_64-linux-gnu\n");
    let observer = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_accounting_vec/observer.c");
    std::fs::copy(observer, directory.join("observer.c")).expect("retained observer source");
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.args([
        "-std=c11",
        "-O0",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-no-pie",
        "-Wl,--build-id=none",
        "-I",
    ])
    .arg(directory)
    .arg(directory.join("observer.c"))
    .arg(directory.join("canonical-runtime.c"))
    .arg(directory.join("program.o"));
    for symbol in SYMBOLS {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    gcc.arg("-Wl,--wrap=malloc")
        .arg("-Wl,--wrap=free")
        .arg("-o")
        .arg(directory.join("program-native"));
    success(gcc, 30);
}
