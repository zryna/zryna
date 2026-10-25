//! Actual canonical runtime processes, including first allocation fault and unreachable second.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
fn success(command: Command) -> std::process::Output {
    let output = process::run(command, 30);
    assert!(
        output.status.success(),
        "{:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}
fn compile(directory: &Path) {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generic_execution_v1_empty_vec_growth");
    for name in ["observer.c", "observer.h"] {
        std::fs::copy(root.join(name), directory.join(name)).expect("retain test observer");
    }
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
    for symbol in ["vec_allocate", "vec_reserve", "vec_release_storage"] {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    gcc.args(["-Wl,--wrap=malloc", "-Wl,--wrap=free"])
        .arg("-o")
        .arg(directory.join("program-native"));
    success(gcc);
}
pub(super) fn all(artifact: &Artifacts) {
    compile(&artifact.directory);
    let mut observations = Vec::new();
    for (name, ordinal, trapped) in [
        ("pristine", 0, false),
        ("fault1", 1, true),
        ("retry1", 0, false),
        ("unreachable-fault2", 2, false),
        ("retry2", 0, false),
    ] {
        observations.push(one(artifact, name, ordinal, trapped));
    }
    std::fs::write(
        artifact.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&observations).expect("actual observations JSON"),
    )
    .expect("retained actual receipts");
}
fn one(artifact: &Artifacts, name: &str, ordinal: u32, trapped: bool) -> serde_json::Value {
    let trace = artifact.directory.join(format!("{name}.actual.jsonl"));
    let mut command = Command::new(artifact.directory.join("program-native"));
    command.arg(ordinal.to_string()).arg(&trace);
    let output = process::run(command, 10);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    if trapped {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "controlled failure after empty-owner cleanup"
        );
        assert!(output.stdout.is_empty());
    } else {
        assert!(output.status.success(), "actual native outcome {:?}", output.status);
        assert_eq!(output.stdout, b"result:18\n");
    }
    let bytes = std::fs::read(&trace).expect("unbuffered actual observer trace");
    assert!(bytes.len() <= 65_536);
    let rows = std::str::from_utf8(&bytes)
        .expect("trace UTF8")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("actual record"))
        .collect::<Vec<_>>();
    check(&rows, trapped);
    std::fs::write(artifact.directory.join(format!("{name}.stdout")), &output.stdout)
        .expect("actual stdout retained");
    serde_json::json!({"name":name,"allocation_fault_ordinal":ordinal,"signal":output.status.signal(),"exit_code":output.status.code(),"records":rows})
}
fn check(rows: &[serde_json::Value], trapped: bool) {
    let names =
        rows.iter().map(|row| row["event"].as_str().expect("actual event")).collect::<Vec<_>>();
    if trapped {
        assert_eq!(
            names,
            [
                "vec-allocate",
                "reserve-entry",
                "attempt",
                "failure",
                "failed-output-poisoned",
                "vec-release"
            ]
        );
        assert_eq!(rows.last().expect("empty Vec cleanup")["id"], 0);
    } else {
        assert_eq!(
            names,
            [
                "vec-allocate",
                "reserve-entry",
                "attempt",
                "host-malloc",
                "reserve-result",
                "acquire",
                "payload-bytes",
                "reserve-entry",
                "reserve-result",
                "vec-release",
                "release"
            ]
        );
        let expected = super::source::row(super::CASE);
        let capacities = rows
            .iter()
            .filter(|r| r["event"] == "reserve-result")
            .map(|r| r["capacity"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            capacities,
            expected["pristine"]["canonical_storage_capacities_after_push"]
                .as_array()
                .expect("source-frozen capacities")
                .clone()
        );
        assert_eq!(rows[6]["bytes"], expected["pristine"]["successful_acquisitions"][0]["bytes"]);
        assert_eq!(
            rows.last().expect("one actual free")["id"],
            expected["pristine"]["release_acquisition_ids"][0]
        );
    }
}
