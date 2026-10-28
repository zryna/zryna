//! Separate actual native processes; each controlled fault must clean up then SIGILL.
use super::{emit::Artifacts, oracle, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
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
        "native child {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected diagnostics: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
pub(super) fn all(case: &str, artifacts: &Artifacts) -> (usize, usize) {
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(success(gcc, 10).stdout, b"x86_64-linux-gnu\n");
    compile(&artifacts.directory);
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(artifacts.directory.join("runtime_metadata.json"))
            .expect("actual checked metadata"),
    )
    .expect("metadata JSON");
    let mut observations = vec![one(case, artifacts, None, "initial", &metadata)];
    let faults = oracle::faults(case);
    for fault in &faults {
        let label = format!("{}-{}", fault.kind, fault.ordinal);
        observations.push(one(case, artifacts, Some(fault), &label, &metadata));
        observations.push(one(case, artifacts, None, &format!("retry-after-{label}"), &metadata));
    }
    assert_eq!(observations.len(), 1 + 2 * faults.len());
    let count = observations.len();
    let receipt = serde_json::json!({"case":case,"target":"native","observations":observations,"all_actual_allocations_released_once":true,"canonical_runtime":true,"count_status_scope":"declared status1/2/3 failure-route controls and actual expired5; no actual u32MAXoverflow claim"});
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("retained actual native observations");
    (count, faults.len())
}
fn compile(directory: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_native_push_upgrade");
    for name in
        ["observer.c", "observer.h", "observer_alloc.h", "observer_counts.h", "observer_host.h"]
    {
        let source = if name == "observer.c" {
            root.join(name)
        } else {
            root.with_file_name("generic_execution_v1_native_affine_push").join(name)
        };
        std::fs::copy(source, directory.join(name)).expect("retain exact test observer sources");
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
    for symbol in SYMBOLS {
        gcc.arg(format!("-Wl,--wrap=zryna_rt_o1_{symbol}"));
    }
    gcc.arg("-Wl,--wrap=malloc").arg("-Wl,--wrap=free");
    gcc.arg("-o").arg(directory.join("program-native"));
    success(gcc, 30);
}
fn one(
    case: &str,
    artifacts: &Artifacts,
    fault: Option<&oracle::Fault>,
    name: &str,
    metadata: &serde_json::Value,
) -> serde_json::Value {
    let trace = artifacts.directory.join(format!("{name}.actual-identities.jsonl"));
    let mut command = Command::new(artifacts.directory.join("program-native"));
    command
        .arg(case)
        .arg(fault.map_or("none", |fault| fault.kind.as_str()))
        .arg(fault.map_or(0, |fault| fault.ordinal).to_string())
        .arg(&trace);
    let output = process::run(command, 10);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    if fault.is_some() {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "exact controlled native trap after observed cleanup"
        );
        assert!(output.stdout.is_empty(), "fault returns no scalar");
    } else {
        assert!(output.status.success());
        assert_eq!(
            output.stdout,
            format!(
                "result:{}\n",
                oracle::row(case)["observation"].as_i64().expect("frozen scalar")
            )
            .as_bytes()
        );
    }
    let bytes = std::fs::read(&trace).expect("unbuffered actual identity trace");
    assert!(bytes.len() <= 65_536);
    let text = String::from_utf8(bytes).expect("canonical-runtime trace UTF8");
    let records = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("actual runtime record"))
        .collect::<Vec<_>>();
    oracle::check(case, fault, &records, metadata);
    std::fs::write(artifacts.directory.join(format!("{name}.stdout")), &output.stdout)
        .expect("actual stdout retained");
    serde_json::json!({"name":name,"outcome":if fault.is_some() {"trap"} else {"return"},"fault":fault.map(|fault|serde_json::json!({"kind":fault.kind,"ordinal":fault.ordinal})),"signal":output.status.signal(),"exit_code":output.status.code(),"records":records,"all_actual_allocations_released_once":true})
}
