//! Separate actual native processes; each controlled fault must clean up then SIGILL.
use super::{emit::Artifacts, oracle, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
const SYMBOLS: [&str; 4] =
    ["string_from_utf8_copy", "string_clone", "string_concat", "string_release"];
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
pub(super) fn native(case: &str, artifacts: &Artifacts) -> (usize, usize) {
    let mut gcc = Command::new("/usr/bin/gcc");
    gcc.arg("-dumpmachine");
    assert_eq!(success(gcc, 10).stdout, b"x86_64-linux-gnu\n");
    compile(&artifacts.directory);
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(artifacts.directory.join("runtime_metadata.json"))
            .expect("actual checked metadata"),
    )
    .expect("metadata JSON");
    let mut observations = Vec::new();
    let mut fault_count = 0;
    for run in oracle::runs(case) {
        let prefix = run["label"].as_str().expect("frozen scalar argument row");
        observations.push(one(
            case,
            &run,
            artifacts,
            None,
            &format!("{prefix}-initial"),
            &metadata,
        ));
        let faults = oracle::faults(&run);
        fault_count += faults.len();
        for fault in &faults {
            let label = format!("{prefix}-{}-{}", fault.kind, fault.ordinal);
            observations.push(one(case, &run, artifacts, Some(fault), &label, &metadata));
            observations.push(one(
                case,
                &run,
                artifacts,
                None,
                &format!("retry-after-{label}"),
                &metadata,
            ));
        }
    }
    assert_eq!(observations.len(), oracle::runs(case).len() + 2 * fault_count);
    let count = observations.len();
    let receipt = serde_json::json!({"case":case,"target":"native","observations":observations,"all_actual_allocations_released_once":true,"canonical_runtime":true,"fault_scope":"real canonical malloc NULL allocation failures; separate declared StringClone failure routes with poisoned unpublished out"});
    std::fs::write(
        artifacts.directory.join("native-observations.json"),
        serde_json::to_vec_pretty(&receipt).expect("native receipt JSON"),
    )
    .expect("retained actual native observations");
    (count, fault_count)
}
fn compile(directory: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/generic_execution_v1_partial_static_targets");
    for name in ["observer.c", "observer.h", "observer_host.h", "observer_strings.h"] {
        let base = if name == "observer.c" {
            root.clone()
        } else {
            root.parent().expect("test directory").join("generic_execution_v1_native_while_runtime")
        };
        std::fs::copy(base.join(name), directory.join(name))
            .expect("retain exact test observer sources");
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
    run: &serde_json::Value,
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
        .arg(&trace)
        .arg(argument(run));
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
                run["pristine"]["scalar_result"].as_i64().expect("frozen scalar")
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
    oracle::check(case, run, fault, &records, metadata);
    std::fs::write(artifacts.directory.join(format!("{name}.stdout")), &output.stdout)
        .expect("actual stdout retained");
    serde_json::json!({"name":name,"arguments":run["arguments"],"native_cli_argument":argument(run),"outcome":if fault.is_some() {"trap"} else {"return"},"fault":fault.map(|fault|serde_json::json!({"kind":fault.kind,"ordinal":fault.ordinal})),"signal":output.status.signal(),"exit_code":output.status.code(),"records":records,"all_actual_allocations_released_once":true})
}

fn argument(run: &serde_json::Value) -> String {
    let arguments = run["arguments"].as_array().expect("frozen source scalar arguments");
    match arguments.as_slice() {
        [] => "none".into(),
        [value] => value.as_bool().map_or_else(
            || {
                i32::try_from(value.as_i64().expect("frozen i32"))
                    .expect("source i32 range")
                    .to_string()
            },
            |value| i32::from(value).to_string(),
        ),
        _ => panic!("only the unchanged finite fixture scalar signatures"),
    }
}
