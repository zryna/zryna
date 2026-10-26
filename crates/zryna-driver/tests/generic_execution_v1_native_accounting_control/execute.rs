//! Actual canonical control8 thresholds with constant-space observations.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
#[path = "compile.rs"]
mod compile;
#[path = "../generic_execution_v1_native_accounting_vec/retention.rs"]
mod retention;
#[path = "summary.rs"]
mod summary;
pub(super) fn all(case: &str, artifacts: &Artifacts) -> usize {
    compile::compile(&artifacts.directory);
    assert_eq!(
        super::emit::digest(super::RUNTIME_ORACLE),
        "536926121f75266cdbc717e442f2efbb6f74a4a43cfc2c0ee31d93967b820c6c"
    );
    let oracle: serde_json::Value = serde_json::from_slice(super::RUNTIME_ORACLE)
        .expect("unchanged control8 pre-output oracle");
    let row = oracle["cases"]
        .as_array()
        .expect("three frozen cases")
        .iter()
        .find(|r| r["case"] == case)
        .expect("exact source case");
    let observations = row["rows"]
        .as_array()
        .expect("six pre-output rows")
        .iter()
        .map(|r| one(case, r, &artifacts.directory))
        .collect::<Vec<_>>();
    let count = observations.len();
    std::fs::write(artifacts.directory.join("native-accounting-control-observations.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "case":case,"actual_invocation_summaries":observations,
        "observer_space":"one physical control block slot and twenty-two lock-free scalar counters",
        "scope":"actual canonical outer control8 entries and malloc/free; compiler private context not read",
        "status_identity_limit":"SIGILL alone does not prove source CAPACITY2; separate same-seal privatecallee owner required"
    })).expect("actual receipt")).expect("retained actual control observations");
    count
}
fn one(case: &str, run: &serde_json::Value, directory: &Path) -> serde_json::Value {
    let label = run["label"].as_str().expect("frozen row");
    let binary = directory.join(format!("{label}.actual-summary.bin"));
    let repeats = run["invocations_same_process"].as_u64().expect("one or two fresh entries");
    let argument = run["arguments"][0].as_i64().expect("source i32 trips");
    let mut command = Command::new(directory.join("program-native"));
    command
        .arg(argument.to_string())
        .arg(repeats.to_string())
        .arg(&binary)
        .arg(run["malloc_fault_ordinal"].as_u64().expect("frozen fault ordinal").to_string());
    std::fs::write(
        directory.join(format!("{label}.frozen-row.json")),
        serde_json::to_vec_pretty(run).expect("frozen row"),
    )
    .expect("retained precise inputs before child");
    let output = process::run(command, 10);
    retention::retain_before_checks(directory, label, argument, repeats, &binary, &output);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let denied = run["outcome"] == "trap";
    if denied {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "ordinary wrapper traps only after held cleanup"
        );
        assert!(output.stdout.is_empty(), "denial publishes no scalar");
    } else {
        assert!(output.status.success(), "{:?}", output.status);
        let line = format!("result:{}\n", run["scalar_result"].as_i64().expect("fixed result"));
        assert_eq!(output.stdout, line.repeat(usize::try_from(repeats).expect("two")).as_bytes());
    }
    let bytes = std::fs::read(&binary).expect("actual summary");
    let actual = summary::check(case, run, repeats, denied, &bytes);
    serde_json::json!({"id":label,"argument":argument,"invocations":repeats,"signal":output.status.signal(),"exit_code":output.status.code(),"actual_fixed_fields":actual})
}
