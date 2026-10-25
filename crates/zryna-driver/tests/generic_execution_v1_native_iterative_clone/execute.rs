//! Actual native children retained before assertions; no code generation is simulated.
use super::process;
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
#[path = "verify.rs"]
mod verify;
fn retained(directory: &Path, label: &str, command: Command, seconds: u64) -> std::process::Output {
    let output = process::run(command, seconds);
    std::fs::write(directory.join(format!("{label}.stdout")), &output.stdout)
        .expect("actual stdout");
    std::fs::write(directory.join(format!("{label}.stderr")), &output.stderr)
        .expect("actual stderr");
    let status = serde_json::json!({"success":output.status.success(),
        "code":output.status.code(),"signal":output.status.signal()});
    std::fs::write(
        directory.join(format!("{label}.status.json")),
        serde_json::to_vec_pretty(&status).expect("actual status"),
    )
    .expect("status before checks");
    output
}
fn compile(directory: &Path, observed: bool) {
    let mut command = Command::new("/usr/bin/gcc");
    command
        .args([
            "-std=c11",
            "-O0",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-no-pie",
            "-Wl,--build-id=none",
            "-I",
        ])
        .arg(directory);
    if observed {
        std::fs::write(directory.join("clone-wrappers.c"), include_bytes!("wrappers.c"))
            .expect("retain exact additive Clone-only wrapper input");
        command.arg(directory.join("observer.c")).arg(directory.join("clone-wrappers.c"));
        let symbols: Vec<String> = serde_json::from_str(include_str!(
            "../generic_execution_v1_native_iterative_drop/symbols.json"
        ))
        .expect("fixed canonical17 test-only wrappers");
        assert_eq!(symbols.len(), 17);
        for name in symbols {
            command.arg(format!("-Wl,--wrap=zryna_rt_o1_{name}"));
        }
        command.arg("-Wl,--wrap=malloc").arg("-Wl,--wrap=free");
    } else {
        command.arg(directory.join("plain.c"));
    }
    command
        .arg(directory.join("canonical-runtime.c"))
        .arg(directory.join("program.o"))
        .arg("-o")
        .arg(directory.join(if observed { "observed" } else { "plain" }));
    let label = if observed { "compile-observed" } else { "compile-plain" };
    let output = retained(directory, label, command, 30);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
}
pub(super) fn ordinary(directory: &Path) {
    compile(directory, false);
    for depth in [0, 1, 8192, 65536] {
        let mut command = Command::new(directory.join("plain"));
        command.arg(depth.to_string());
        let output = retained(directory, &format!("plain-{depth}"), command, 20);
        assert!(
            output.status.success(),
            "actual ordinary native depth{depth}: {:?}",
            output.status
        );
        assert_eq!(output.stdout, b"result:7\nresult:7\n");
        assert!(output.stderr.is_empty());
    }
}
fn one(directory: &Path, depth: u32, seed: u32, fault: bool, label: &str) -> std::process::Output {
    let trace = directory.join(format!("{label}.trace.bin"));
    let mut command = Command::new(directory.join("observed"));
    command
        .arg(depth.to_string())
        .arg(seed.to_string())
        .arg(u32::from(fault).to_string())
        .arg(trace);
    retained(directory, label, command, 20)
}
pub(super) fn observed(directory: &Path) {
    compile(directory, true);
    for depth in [0, 1, 8192, 65536] {
        let label = format!("clone-pristine-{depth}");
        let output = one(directory, depth, 0, false, &label);
        verify::check(directory, &label, depth, false, &output);
        let label = format!("clone-leaf-fault-{depth}");
        let output = one(directory, depth, 0, true, &label);
        verify::check(directory, &label, depth, true, &output);
        let label = format!("clone-pristine-process-retry-{depth}");
        let output = one(directory, depth, 0, false, &label);
        verify::check(directory, &label, depth, false, &output);
    }
}
