//! Real identity traces, pristine retries, and separately spawned SIGILL observations.
use super::{emit::Artifacts, oracle, process};
use std::{os::unix::process::ExitStatusExt, path::PathBuf, process::Command};

pub(super) struct Counts {
    pub(super) observations: [usize; 3],
    pub(super) native_faults: usize,
}
fn root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_struct_clone_v5")
}
fn successful(command: Command, seconds: u64) -> std::process::Output {
    successful_output(process::run(command, seconds))
}
fn successful_output(output: std::process::Output) -> std::process::Output {
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
    let allocations = oracle::allocations(case);
    let expected_count = 1 + 2 * allocations;
    let mut counts = Counts { observations: [0; 3], native_faults: 0 };
    for (index, target) in ["javascript", "webassembly"].into_iter().enumerate() {
        let mut node = Command::new("node");
        node.arg(root().join("portable.mjs")).arg(target).arg(&artifacts.directory).arg(case);
        let output =
            successful_output(super::portable_observation::run(node, target, case, artifacts));
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("portable observation JSON");
        assert_eq!(value["target"], target);
        assert_eq!(value["case"], case);
        assert_eq!(value["actualAllocationsReleasedExactlyOnce"], true);
        let observations =
            value["observations"].as_array().expect("retained individual observations");
        assert_eq!(observations.len(), expected_count);
        for (ordinal, observation) in observations.iter().enumerate() {
            let fault = if ordinal % 2 == 1 { ordinal.div_ceil(2) } else { 0 };
            assert_eq!(observation["fault"], fault);
            assert_eq!(observation["events"], serde_json::json!(oracle::events(case, fault)));
            assert_eq!(observation["outcome"], if fault == 0 { "return" } else { "trap" });
            if fault == 0 {
                assert_eq!(observation["result"], 7);
            } else {
                assert!(observation.get("result").is_none(), "fault must not return a scalar");
            }
        }
        counts.observations[index] = observations.len();
        std::fs::write(
            artifacts.directory.join(format!("{target}-observations.json")),
            output.stdout,
        )
        .expect("retained portable observations and identities");
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
    let mut observations = vec![native_case(case, artifacts, 0, "initial")];
    for fault in 1..=allocations {
        observations.push(native_case(case, artifacts, fault, &format!("fault-{fault}")));
        counts.native_faults += 1;
        observations.push(native_case(case, artifacts, 0, &format!("retry-after-{fault}")));
    }
    assert_eq!(observations.len(), expected_count);
    counts.observations[2] = observations.len();
    let receipt = serde_json::json!({"target":"native","case":case,
        "actualAllocationsReleasedExactlyOnce":true,"observations":observations});
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("retained native observations including every pristine retry");
    counts
}
fn native_case(
    case: &str,
    artifacts: &Artifacts,
    fault: usize,
    observation: &str,
) -> serde_json::Value {
    let mut command = Command::new(artifacts.directory.join("program-native"));
    let identity_path = artifacts.directory.join(format!("native-{observation}.identities.jsonl"));
    command.arg(fault.to_string()).arg(oracle::allocations(case).to_string()).arg(&identity_path);
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
    let identity_bytes = std::fs::read(&identity_path).expect("actual native pointer records");
    assert!(identity_bytes.len() <= 65_536, "bounded identity evidence");
    let identity_text = String::from_utf8(identity_bytes).expect("native identity JSONL");
    let identities = identity_text
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("native identity record")
        })
        .collect::<Vec<_>>();
    super::native_identity::check(case, fault, &identities);
    let mut expected = oracle::events(case, fault);
    if fault == 0 {
        expected.push("result:7".into());
    }
    let stdout = String::from_utf8(output.stdout.clone()).expect("native ASCII events");
    assert_eq!(
        stdout,
        format!("{}\n", expected.join("\n")),
        "exact cleanup stdout before native trap"
    );
    std::fs::write(
        artifacts.directory.join(format!("native-{observation}.stdout")),
        &output.stdout,
    )
    .expect("individual native event evidence");
    serde_json::json!({"observation":observation,"fault":fault,
        "outcome":if fault == 0 { "return" } else { "trap" },
        "exit_code":output.status.code(),"signal":output.status.signal(),"stdout":stdout, "identities":identities, "actualAllocationsReleasedExactlyOnce":true})
}
