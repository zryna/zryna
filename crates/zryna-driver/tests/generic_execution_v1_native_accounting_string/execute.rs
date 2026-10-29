//! Actual event thresholds with fixed-size summaries, not seeded compiler counters.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
const SYMBOLS: [&str; 3] = ["string_from_utf8_copy", "string_clone", "string_release"];
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
pub(super) fn all(case: &str, artifacts: &Artifacts) -> usize {
    compile(&artifacts.directory);
    assert_eq!(
        super::emit::digest(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/m7-generic-execution-native-accounting-string/RUNTIME_EXPECTED_FREEZE.json"
        ))),
        "fe2636923a20fd8ca2c6fb176aedc0eaf91d607bcd584555be335c38c4dee490"
    );
    let oracle: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/m7-generic-execution-native-accounting-string/RUNTIME_EXPECTED_FREEZE.json"
    )))
    .expect("independent pre-output threshold oracle");
    let mut observations = Vec::new();
    for run in oracle["runs"].as_array().expect("fourteen frozen threshold rows") {
        if run["case"] == case {
            observations.push(one(run, &artifacts.directory));
        }
    }
    let count = observations.len();
    std::fs::write(
        artifacts.directory.join("native-accounting-observations.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "case":case,"actual_invocation_summaries":observations,
            "observer_space":"two physical block slots and fourteen lock-free scalar counters",
            "scope":"actual canonical entries and malloc/free; compiler private context not observed",
            "status_identity_limit":"SIGILL alone does not prove source CAPACITY2; separate genuine effect/failure owner required"
        })).expect("actual observation JSON"),
    ).expect("retained actual observations");
    count
}
fn compile(directory: &Path) {
    let mut machine = Command::new("/usr/bin/gcc");
    machine.arg("-dumpmachine");
    assert_eq!(success(machine, 10).stdout, b"x86_64-linux-gnu\n");
    let observer = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_accounting_string/observer.c");
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
fn one(run: &serde_json::Value, directory: &Path) -> serde_json::Value {
    let label = run["id"].as_str().expect("fixed row identifier");
    let summary = directory.join(format!("{label}.actual-summary.bin"));
    let repeats = run["invocations_in_same_process"].as_u64().expect("one or two invocations");
    let argument = run["entry_arguments_per_invocation"][0].as_i64().expect("frozen trips");
    let mut command = Command::new(directory.join("program-native"));
    command.arg(argument.to_string()).arg(repeats.to_string()).arg(&summary);
    let output = process::run(command, 10);
    retain_before_checks(directory, label, argument, repeats, &summary, &output);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let expected = &run["expected_per_invocation"];
    let denied = !expected["scalar_stdout_published"].as_bool().expect("publication flag");
    if denied {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "ordinary wrapper traps after cleanup"
        );
        assert!(output.stdout.is_empty(), "global denial publishes no scalar");
    } else {
        assert!(output.status.success(), "{:?}", output.status);
        let line = format!("result:{}\n", expected["result"].as_i64().expect("scalar result"));
        assert_eq!(output.stdout, line.repeat(usize::try_from(repeats).expect("two")).as_bytes());
    }
    let bytes = std::fs::read(&summary).expect("actual fixed-size summary");
    assert_eq!(bytes.len(), usize::try_from(repeats).expect("two") * 14 * 8);
    let mut actual = Vec::new();
    for (index, record) in bytes.chunks_exact(14 * 8).enumerate() {
        let fields = record
            .chunks_exact(8)
            .map(|field| u64::from_le_bytes(field.try_into().expect("u64")))
            .collect::<Vec<_>>();
        assert_eq!(fields[0], 0x4d34_3136_4143_4331);
        assert_eq!(fields[1], if denied { 1 } else { 2 });
        assert_eq!(fields[2], u64::try_from(index).expect("bounded") + 1);
        for (slot, name) in [
            (3, "admitted_outer_string_from"),
            (4, "admitted_outer_string_clone"),
            (5, "admitted_outer_string_release"),
            (6, "positive_backing_attempts"),
            (7, "successful_allocations"),
            (8, "physical_frees"),
            (9, "live_at_boundary"),
            (10, "maximum_live_blocks"),
            (12, "logical_status_events"),
        ] {
            assert_eq!(
                fields[slot],
                expected[name].as_u64().expect("pre-output scalar"),
                "{label}: {name}"
            );
        }
        assert_eq!(fields[11], 0, "all String owners cleaned at invocation boundary");
        assert_eq!(
            fields[13],
            fields[3] + fields[4],
            "out-zero check before each admitted acquisition"
        );
        actual.push(fields);
    }
    std::fs::write(directory.join(format!("{label}.stdout")), &output.stdout)
        .expect("retained stdout");
    serde_json::json!({"id":label,"argument":argument,"invocations":repeats,"signal":output.status.signal(),"exit_code":output.status.code(),"actual_fixed_fields":actual})
}

fn retain_before_checks(
    directory: &Path,
    label: &str,
    argument: i64,
    repeats: u64,
    summary: &Path,
    output: &std::process::Output,
) {
    std::fs::write(directory.join(format!("{label}.stdout")), &output.stdout)
        .expect("retain actual stdout before expectations");
    std::fs::write(directory.join(format!("{label}.stderr")), &output.stderr)
        .expect("retain actual stderr before expectations");
    let binary = match std::fs::read(summary) {
        Ok(bytes) => serde_json::json!({
            "path":summary,"bytes":bytes.len(),"sha256":super::emit::digest(&bytes)
        }),
        Err(error) => serde_json::json!({"path":summary,"read_error":error.to_string()}),
    };
    let path = directory.join(format!("{label}.actual-child-before-checks.json"));
    let receipt = serde_json::json!({
        "id":label,"argument":argument,"invocations_requested":repeats,
        "expectations_checked":false,
        "status":{"display":output.status.to_string(),"exit_code":output.status.code(),
            "signal":output.status.signal(),"core_dumped":output.status.core_dumped()},
        "stdout":{"bytes":output.stdout.len(),"sha256":super::emit::digest(&output.stdout)},
        "stderr":{"bytes":output.stderr.len(),"sha256":super::emit::digest(&output.stderr)},
        "binary_summary":binary
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&receipt).expect("actual child receipt"))
        .expect("retain child evidence before expectations");
    println!("native accounting child evidence before checks: {}", path.display());
}
