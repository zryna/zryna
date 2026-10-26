//! Real runtime allocation traces and native cleanup-before-SIGILL observations.
use super::{emit::Artifacts, oracle, process, source::Case};
use std::{os::unix::process::ExitStatusExt, path::PathBuf, process::Command};

fn root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_struct_v4")
}
fn successful(command: Command, seconds: u64) -> std::process::Output {
    let output = process::run(command, seconds);
    assert!(
        output.status.success(),
        "child failed: {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected runtime diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
pub(super) fn check_tools() {
    let mut node = Command::new("node");
    node.arg("--version");
    assert_eq!(successful(node, 10).stdout, b"v22.22.1\n", "repository-pinned Node");
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(successful(gcc, 10).stdout, b"x86_64-linux-gnu\n");
}
pub(super) fn all_targets(case: Case, artifacts: &Artifacts) {
    for target in ["javascript", "webassembly"] {
        let mut node = Command::new("node");
        node.arg(root().join("portable.mjs"))
            .arg(target)
            .arg(&artifacts.directory)
            .arg(if case.owned() { "owned" } else { "copy" });
        let output = successful(node, 10);
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("portable observation JSON");
        assert_eq!(value["target"], target);
        assert_eq!(value["observations"], if case.owned() { 7 } else { 1 });
        assert_eq!(value["actualAllocationsReleasedExactlyOnce"], true);
        std::fs::write(
            artifacts.directory.join(format!("{target}-observations.json")),
            output.stdout,
        )
        .expect("portable observations");
    }
    let executable = artifacts.directory.join("program-native");
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
    .arg(&artifacts.directory)
    .arg(root().join("runtime.c"))
    .arg(artifacts.directory.join("program.o"))
    .arg("-o")
    .arg(&executable);
    successful(gcc, 30);
    let mut observations = vec![native_case(case, artifacts, 0)];
    if case.owned() {
        for fault in 1..=3 {
            observations.push(native_case(case, artifacts, fault));
            observations.push(native_case(case, artifacts, 0));
        }
    }
    let receipt = serde_json::json!({ "target":"native", "observations":observations });
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("native observations including every pristine retry");
}
fn native_case(case: Case, artifacts: &Artifacts, fault: usize) -> serde_json::Value {
    let mut command = Command::new(artifacts.directory.join("program-native"));
    command.arg(fault.to_string()).arg(if case.owned() { "3" } else { "0" });
    let output = process::run(command, 10);
    assert!(
        output.stderr.is_empty(),
        "native runtime diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if fault == 0 {
        assert!(output.status.success());
    } else {
        assert_eq!(output.status.signal(), Some(libc::SIGILL), "controlled allocation trap");
    }
    let mut expected =
        oracle::events(case.owned(), fault).iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    if fault == 0 {
        expected.push(format!("result:{}", oracle::VALUE));
    }
    assert_eq!(
        String::from_utf8(output.stdout.clone())
            .expect("native ASCII events")
            .lines()
            .collect::<Vec<_>>(),
        expected,
        "actual cleanup precedes native trap"
    );
    std::fs::write(
        artifacts.directory.join(format!("native-fault-{fault}.stdout")),
        &output.stdout,
    )
    .expect("native event evidence");
    serde_json::json!({
        "fault":fault,"exit_code":output.status.code(),"signal":output.status.signal(),
        "stdout":String::from_utf8(output.stdout).expect("native ASCII receipt"),
    })
}
