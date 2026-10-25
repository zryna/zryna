//! Bounded native subprocesses; raw observations persist before any expected-result assertion.
use super::{emit::Artifacts, oracle, process};
use serde_json::{Value, json};
use std::{io::Read as _, os::unix::process::ExitStatusExt, path::Path, process::Command};
const SYMBOLS: [&str; 17] = [
    "allocate",
    "grow",
    "release",
    "string_from_utf8_copy",
    "string_clone",
    "string_concat",
    "string_release",
    "vec_allocate",
    "vec_reserve",
    "vec_release_storage",
    "strong_clone",
    "weak_downgrade",
    "weak_clone",
    "weak_upgrade",
    "strong_release_begin",
    "strong_release_finish",
    "weak_release",
];
fn success(command: Command, seconds: u64) -> std::process::Output {
    let output = process::run(command, seconds);
    assert!(
        output.status.success(),
        "child {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}
pub(super) fn all(case: &str, artifacts: &Artifacts) -> (usize, usize, usize) {
    compile(&artifacts.directory);
    let metadata: Value = serde_json::from_slice(
        &std::fs::read(artifacts.directory.join("runtime_metadata.json"))
            .expect("actual native metadata"),
    )
    .expect("native metadata JSON");
    let expected = oracle::case(case);
    let normal = expected["normal"].as_array().expect("frozen normal rows");
    let faults = expected["faults"].as_array().expect("frozen ALLOCATION1 rows");
    let mut observations = Vec::new();
    let mut traps = 0;
    for (index, run) in normal.iter().enumerate() {
        for repeat in 0..2 {
            observations.push(one(
                run,
                artifacts,
                &format!("normal-{index}-repeat-{repeat}"),
                &metadata,
            ));
            traps += usize::from(run.get("trap").is_some());
        }
    }
    for (index, fault) in faults.iter().enumerate() {
        observations.push(one(fault, artifacts, &format!("allocation-fault-{index}"), &metadata));
        traps += 1;
        let retry = normal
            .iter()
            .find(|row| row["arguments"] == fault["arguments"])
            .expect("same exact argument pristine retry");
        assert!(
            retry.get("trap").is_none(),
            "frozen faults target valid successful source arguments"
        );
        observations.push(one(retry, artifacts, &format!("retry-after-{index}"), &metadata));
    }
    let count = observations.len();
    std::fs::write(artifacts.directory.join("native-observations.json"), serde_json::to_vec_pretty(&json!({
        "case":case,"target":"native","observations":observations,"controlled_allocation_faults":faults.len(),
        "SIGILL_traps_including_bounds":traps,"same_arguments_pristine_retry":true,"canonical17_unchanged":true,
        "oracle_projection":"unchanged shared source trace oracle; native physical extents independently checked from same NEWseal"})).expect("receipt JSON")).expect("actual receipt");
    (count, traps, faults.len())
}
fn compile(directory: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_indexed_runtime");
    for name in [
        "observer.c",
        "observer.h",
        "observer_host.h",
        "observer_strings.h",
        "observer_vectors.h",
        "observer_unused.h",
    ] {
        std::fs::copy(root.join(name), directory.join(name)).expect("retained observer source");
    }
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(success(gcc, 10).stdout, b"x86_64-linux-gnu\n");
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
fn one(run: &Value, artifacts: &Artifacts, name: &str, metadata: &Value) -> Value {
    let trace = artifacts.directory.join(format!("{name}.actual-identities.jsonl"));
    let args = arguments(run);
    let mut command = Command::new(artifacts.directory.join("program-native"));
    command
        .arg(run["fault"]["ordinal"].as_u64().unwrap_or(0).to_string())
        .arg(&trace)
        .arg(args[0].to_string())
        .arg(args[1].to_string());
    let output = process::run(command, 10);
    std::fs::write(artifacts.directory.join(format!("{name}.stdout")), &output.stdout)
        .expect("raw stdout before assertion");
    std::fs::write(artifacts.directory.join(format!("{name}.stderr")), &output.stderr)
        .expect("raw stderr before assertion");
    let status = json!({"signal":output.status.signal(),"exit_code":output.status.code(),"expected_source_row":run});
    std::fs::write(
        artifacts.directory.join(format!("{name}.status.json")),
        serde_json::to_vec_pretty(&status).expect("process JSON"),
    )
    .expect("raw status before assertion");
    let mut bytes = Vec::new();
    std::fs::File::open(trace)
        .expect("unbuffered observed trace")
        .take(65_537)
        .read_to_end(&mut bytes)
        .expect("bounded trace read");
    assert!(bytes.len() <= 65_536, "finite trace ceiling");
    let records = std::str::from_utf8(&bytes)
        .expect("trace UTF8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("actual canonical event JSON"))
        .collect::<Vec<_>>();
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    if run.get("trap").is_some() {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "exact bounds/allocation native controlled trap"
        );
        assert!(output.stdout.is_empty(), "failed source returns no scalar");
    } else {
        assert!(output.status.success());
        assert_eq!(
            output.stdout,
            format!("result:{}\n", run["result"].as_i64().expect("frozen scalar")).as_bytes()
        );
    }
    let checked = oracle::check(run, &records, metadata);
    json!({"name":name,"arguments":run["arguments"],"status":status,"records":records,"checked":checked})
}
fn arguments(run: &Value) -> [i32; 2] {
    let args = run["arguments"].as_array().expect("frozen scalar arguments");
    assert!(args.len() <= 2);
    let mut result = [0; 2];
    for (slot, value) in result.iter_mut().zip(args) {
        *slot = value.as_bool().map_or_else(
            || i32::try_from(value.as_i64().expect("frozen i32")).expect("source i32"),
            i32::from,
        );
    }
    result
}
