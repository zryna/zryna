//! Actual canonical malloc/free accounting and SIGILL bounds, followed by a pristine process.
use super::process;
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};

pub(super) fn check(directory: &Path, expected: &serde_json::Value) {
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
            "-Wl,--wrap=malloc",
            "-Wl,--wrap=free",
            "-I",
        ])
        .arg(directory)
        .arg(directory.join("observer.c"))
        .arg(directory.join("canonical-runtime.c"))
        .arg(directory.join("program.o"))
        .arg("-o")
        .arg(directory.join("program-native"));
    let compiled = process::run(command, 30);
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    assert!(compiled.stderr.is_empty());
    let runs = expected["runs"].as_array().expect("pre-output source runs");
    let pristine = runs.iter().find(|r| r["bounds_trap"] == false).expect("success row");
    let mut actual = Vec::new();
    for run in runs {
        actual.push(one(directory, run));
        if run["bounds_trap"] == true {
            actual.push(one(directory, pristine));
        }
    }
    std::fs::write(
        directory.join("native-actual.json"),
        serde_json::to_vec_pretty(&actual).expect("actual native JSON"),
    )
    .expect("retain native observations");
}

fn one(directory: &Path, run: &serde_json::Value) -> serde_json::Value {
    let mut command = Command::new(directory.join("program-native"));
    for arg in run["arguments"].as_array().expect("exact scalar args") {
        command.arg(arg.as_i64().expect("i32 source input").to_string());
    }
    let output = process::run(command, 10);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    if run["bounds_trap"] == true {
        assert_eq!(output.status.signal(), Some(4), "actual SIGILL, no assertion/host failure");
    } else {
        assert!(output.status.success());
    }
    let text = std::str::from_utf8(&output.stdout).expect("actual JSON trace");
    let rows = text
        .lines()
        .map(|s| serde_json::from_str::<serde_json::Value>(s).expect("complete actual event"))
        .collect::<Vec<_>>();
    let mut live = std::collections::BTreeSet::new();
    let mut count = 0u64;
    let mut returned = None;
    for row in &rows {
        if let Some(id) = row["acquire"].as_u64() {
            assert!(live.insert(id));
            count += 1;
        } else if let Some(id) = row["release"].as_u64() {
            assert!(live.remove(&id), "release must match one actual live base");
        } else {
            assert!(returned.is_none());
            returned = Some(row["returned"].as_i64().expect("actual source result"));
        }
    }
    assert!(
        live.is_empty(),
        "all acquired native storage ends before return or controlled bounds trap"
    );
    assert_eq!(count, run["native_allocations"].as_u64().expect("source-only allocation count"));
    assert_eq!(returned, run["result"].as_i64());
    serde_json::json!({"arguments":run["arguments"],"returned":returned,"signal":output.status.signal(),"events":rows,"allocations_balanced":true})
}
