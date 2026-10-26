//! Real canonical8 wrapping, unchanged ordinary input and compiler-owned static metadata.
use super::super::{emit, process};
use std::{path::Path, process::Command};
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
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    std::fs::copy(
        tests.join("generic_execution_v1_native_accounting_control/observer.c"),
        directory.join("control_observer.c"),
    )
    .expect("exact unchanged canonical control observer");
    std::fs::copy(
        tests.join("generic_execution_v1_native_control_private_accounting/private_probe.c"),
        directory.join("private_probe.c"),
    )
    .expect("private test-only projection source");
    let object = std::fs::read(directory.join("program.o")).expect("ordinary input before link");
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.args([
        "-std=c11",
        "-O0",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-no-pie",
        "-Wl,--build-id=none",
        "-Wl,--no-relax",
        "-I",
    ])
    .arg(directory)
    .arg(directory.join("private_probe.c"))
    .arg(directory.join("canonical-runtime.c"))
    .arg(directory.join("program.o"));
    for symbol in [
        "allocate",
        "strong_clone",
        "weak_downgrade",
        "weak_clone",
        "weak_upgrade",
        "strong_release_begin",
        "strong_release_finish",
        "weak_release",
    ] {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    gcc.arg("-Wl,--wrap=malloc")
        .arg("-Wl,--wrap=free")
        .arg("-o")
        .arg(directory.join("private-program-native"));
    success(gcc, 30);
    assert_eq!(std::fs::read(directory.join("program.o")).expect("unchanged linked input"), object);
    let inputs = [
        "program.o",
        "program.zir",
        "private_binding.h",
        "private-binding.json",
        "control_observer.c",
        "private_probe.c",
        "canonical-runtime.c",
        "PRIVATE_EXPECTED_FREEZE.json",
        "private-program-native",
    ]
    .into_iter()
    .map(|name| {
        let bytes = std::fs::read(directory.join(name)).expect("retained real probe input");
        (name, emit::digest(&bytes))
    })
    .collect::<std::collections::BTreeMap<_, _>>();
    std::fs::write(
        directory.join("private-inputs-before-execution.json"),
        serde_json::to_vec_pretty(&inputs).expect("bounded inputs"),
    )
    .expect("retain before child process");
}
