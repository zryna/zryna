//! Bounded private child observations; all outputs/status persist before semantic checks.
use serde_json::Value;
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
fn retained(directory: &Path, label: &str, command: Command, seconds: u64) -> std::process::Output {
    let output = super::super::process::run(command, seconds);
    for (suffix, bytes) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
        std::fs::write(directory.join(format!("{label}.{suffix}")), bytes)
            .expect("actual streams first");
    }
    let status = serde_json::json!({"success":output.status.success(),"code":output.status.code(),
        "signal":output.status.signal()});
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
        for name in ["observer.c", "payload.c", "wrappers.c"] {
            command.arg(directory.join(format!("mixed-{name}")));
        }
        let symbols: Vec<String> = serde_json::from_str(include_str!(
            "../../generic_execution_v1_native_iterative_drop/symbols.json"
        ))
        .expect("same17symbols");
        assert_eq!(symbols.len(), 17);
        for symbol in symbols {
            command.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
        }
        command.args(["-Wl,--wrap=malloc", "-Wl,--wrap=free"]);
    } else {
        command.arg(directory.join("mixed-plain.c"));
    }
    command
        .arg(directory.join("canonical-runtime.c"))
        .arg(directory.join("program.o"))
        .arg("-o")
        .arg(directory.join(if observed { "mixed-observed" } else { "mixed-plain" }));
    let output = retained(
        directory,
        if observed { "mixed-compile-observed" } else { "mixed-compile-plain" },
        command,
        30,
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
}
pub(super) fn all(directory: &Path, oracle: &Value, addendum: &Value) {
    compile(directory, false);
    let ordinary =
        retained(directory, "mixed-ordinary", Command::new(directory.join("mixed-plain")), 20);
    assert!(ordinary.status.success());
    assert!(ordinary.stderr.is_empty());
    assert_eq!(ordinary.stdout, b"result:97\nresult:97\n");
    compile(directory, true);
    one(directory, "mixed-pristine", oracle, addendum, None);
    let clones = oracle["StringClone_faults"].as_array().expect("9Clone rows");
    let allocs = oracle["Allocate_faults"].as_array().expect("26physical rows");
    assert_eq!((clones.len(), allocs.len()), (9, 26));
    for fault in clones.iter().chain(allocs) {
        let label = format!(
            "mixed-{}-{}",
            fault["kind"].as_str().expect("fault kind"),
            fault["ordinal"].as_u64().expect("ordinal")
        );
        one(directory, &label, oracle, addendum, Some(fault));
        one(directory, &format!("{label}-pristine-process-retry"), oracle, addendum, None);
    }
}
fn one(directory: &Path, label: &str, oracle: &Value, addendum: &Value, fault: Option<&Value>) {
    let mut command = Command::new(directory.join("mixed-observed"));
    command
        .arg(fault.map_or("none", |row| row["kind"].as_str().expect("kind")))
        .arg(fault.map_or(0, |row| row["ordinal"].as_u64().expect("ordinal")).to_string())
        .arg(directory.join(format!("{label}.bin")))
        .arg(directory.join(format!("{label}.jsonl")));
    let output = retained(directory, label, command, 20);
    super::verify::check(directory, label, oracle, addendum, fault, &output);
}
