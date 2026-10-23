//! Three additional zero-allocation observations, using the existing bounded runtime helpers.
use super::{emit::Artifacts, oracle, process};
use std::{path::PathBuf, process::Command};
pub(super) struct Counts {
    pub(super) observations: [usize; 3],
    pub(super) native_faults: usize,
}
fn root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_struct_v4_imported")
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
pub(super) fn all_targets(case: &str, artifacts: &Artifacts) -> Counts {
    assert_eq!(case, "imported-alias-decoy", "honest independent imported identity");
    assert_eq!(oracle::allocations(case), 0);
    assert!(oracle::events(case, 0).is_empty());
    for target in ["javascript", "webassembly"] {
        let mut node = Command::new("node");
        node.arg(root().join("portable.mjs")).arg(target).arg(&artifacts.directory).arg(case);
        let output = successful(node, 10);
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("portable observation JSON");
        assert_eq!(value["target"], target);
        assert_eq!(value["case"], case);
        assert_eq!(value["actualAllocationsReleasedExactlyOnce"], true);
        let observations = value["observations"].as_array().expect("individual observation");
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation["fault"], 0);
        assert_eq!(observation["outcome"], "return");
        assert_eq!(observation["result"], 7);
        assert_eq!(observation["calls"], 0);
        for field in ["events", "allocated", "released"] {
            assert_eq!(observation[field], serde_json::json!([]));
        }
        std::fs::write(
            artifacts.directory.join(format!("{target}-observations.json")),
            output.stdout,
        )
        .expect("retained imported portable observation");
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
    .arg(root().join("../generic_struct_v4_nested/runtime.c"))
    .arg(artifacts.directory.join("program.o"))
    .arg("-o")
    .arg(&executable);
    successful(gcc, 30);
    let identity_path = artifacts.directory.join("native-initial.identities.jsonl");
    let mut command = Command::new(&executable);
    command.arg("0").arg("0").arg(&identity_path);
    let output = successful(command, 10);
    assert_eq!(output.stdout, b"result:7\n");
    assert!(std::fs::read(&identity_path).expect("actual native identity records").is_empty());
    std::fs::write(artifacts.directory.join("native-initial.stdout"), &output.stdout)
        .expect("retained imported native stdout");
    let receipt = serde_json::json!({"target":"native","case":case,
        "actualAllocationsReleasedExactlyOnce":true,
        "observations":[{"observation":"initial","fault":0,"outcome":"return","result":7,
            "calls":0,"events":[],"allocated":[],"released":[],
            "exit_code":output.status.code(),"stdout":"result:7\n"}]});
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("retained imported native observation");
    Counts { observations: [1, 1, 1], native_faults: 0 }
}
