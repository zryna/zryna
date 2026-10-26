//! Actual canonical Vec thresholds with constant-space observations.
use super::{emit::Artifacts, process};
use std::{os::unix::process::ExitStatusExt, path::Path, process::Command};
#[path = "compile.rs"]
mod compile;
#[path = "retention.rs"]
mod retention;
#[path = "summary.rs"]
mod summary;
pub(super) fn all(case: &str, artifacts: &Artifacts) -> usize {
    compile::compile(&artifacts.directory);
    assert_eq!(
        super::emit::digest(super::RUNTIME_ORACLE),
        "74e37c21295455022586c9115c6c1ed3b9e6975c1864304241660d5b55949a64"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(super::RUNTIME_ORACLE).expect("unchanged Vec pre-output oracle");
    let row = oracle["cases"]
        .as_array()
        .expect("three frozen cases")
        .iter()
        .find(|r| r["case"] == case)
        .expect("exact source case");
    let observations = row["rows"]
        .as_array()
        .expect("five pre-output rows")
        .iter()
        .map(|r| one(r, &artifacts.directory))
        .collect::<Vec<_>>();
    let count = observations.len();
    std::fs::write(artifacts.directory.join("native-accounting-vec-observations.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "case":case,"actual_invocation_summaries":observations,
        "observer_space":"two physical block slots and fourteen lock-free scalar counters",
        "scope":"actual canonical outer Vec entries and malloc/free; compiler private context not read",
        "status_identity_limit":"SIGILL alone does not prove source CAPACITY2; separate same-seal privatecallee owner required"
    })).expect("actual receipt")).expect("retained actual Vec observations");
    count
}
fn one(run: &serde_json::Value, directory: &Path) -> serde_json::Value {
    let label = run["label"].as_str().expect("frozen row");
    let binary = directory.join(format!("{label}.actual-summary.bin"));
    let repeats = run["invocations_same_process"].as_u64().expect("one or two fresh entries");
    let argument = run["arguments"][0].as_i64().expect("source i32 trips");
    let mut command = Command::new(directory.join("program-native"));
    command.arg(argument.to_string()).arg(repeats.to_string()).arg(&binary);
    let output = process::run(command, 10);
    retention::retain_before_checks(directory, label, argument, repeats, &binary, &output);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let denied = run["outcome"] == "trap";
    if denied {
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGILL),
            "ordinary wrapper traps after held cleanup"
        );
        assert!(output.stdout.is_empty(), "denial publishes no scalar");
    } else {
        assert!(output.status.success(), "{:?}", output.status);
        let line = format!("result:{}\n", run["scalar_result"].as_i64().expect("fixed result"));
        assert_eq!(output.stdout, line.repeat(usize::try_from(repeats).expect("two")).as_bytes());
    }
    let bytes = std::fs::read(&binary).expect("actual summary");
    let actual = summary::check(run, repeats, denied, &bytes);
    serde_json::json!({"id":label,"argument":argument,"invocations":repeats,"signal":output.status.signal(),"exit_code":output.status.code(),"actual_fixed_fields":actual})
}
