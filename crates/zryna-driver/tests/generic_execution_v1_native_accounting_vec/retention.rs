//! Retain actual child evidence before any expectation assertion.
use std::{os::unix::process::ExitStatusExt, path::Path};
pub(super) fn retain_before_checks(
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
            "path":summary,"bytes":bytes.len(),"sha256":crate::emit::digest(&bytes)
        }),
        Err(error) => serde_json::json!({"path":summary,"read_error":error.to_string()}),
    };
    let path = directory.join(format!("{label}.actual-child-before-checks.json"));
    let receipt = serde_json::json!({
        "id":label,"argument":argument,"invocations_requested":repeats,
        "expectations_checked":false,
        "status":{"display":output.status.to_string(),"exit_code":output.status.code(),
            "signal":output.status.signal(),"core_dumped":output.status.core_dumped()},
        "stdout":{"bytes":output.stdout.len(),"sha256":crate::emit::digest(&output.stdout)},
        "stderr":{"bytes":output.stderr.len(),"sha256":crate::emit::digest(&output.stderr)},
        "binary_summary":binary
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&receipt).expect("actual child receipt"))
        .expect("retain child evidence before expectations");
    println!("native accounting child evidence before checks: {}", path.display());
}
