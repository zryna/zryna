//! Exact moved ordinary String source/call frontiers and their unchanged oracle.
#[path = "../generic_execution_v1_native_accounting_string/source.rs"]
mod source;
use sha2::{Digest, Sha256};
use std::process::Command;
#[path = "../../../zryna-backend-javascript/src/generic_execution_v1/accounting_tests/process.rs"]
pub(super) mod process;
#[test]
fn positive_attempt_frontier_survives_source_calls_and_same_process_entry_reset() {
    actual("nonempty-call-loop", 5);
}
#[test]
fn empty_string_status_frontier_needs_no_physical_attempt_or_iteration_bound() {
    actual("empty-call-loop", 5);
}
#[test]
fn denied_callee_clone_cleans_the_retained_parent_and_retries_pristine() {
    actual("retained-clone-call-loop", 4);
}
fn actual(case: &str, rows: usize) {
    let oracle = include_bytes!(
        "../../../zryna-backend-javascript/src/generic_execution_v1/accounting_tests/RUNTIME_EXPECTED_FREEZE.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "fe2636923a20fd8ca2c6fb176aedc0eaf91d607bcd584555be335c38c4dee490"
    );
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let artifact = super::emit(program)
                .expect("ordinary genuine NEW target accepts this resource cycle");
            assert_eq!(artifact, super::emit(program).expect("deterministic ordinary byte owner"));
            assert!(artifact.source.contains("const $runtimeManifest="));
            for forbidden in [
                "globalThis",
                "Symbol.for",
                "import ",
                "import(",
                "require(",
                "eval(",
                "new Function",
            ] {
                assert!(!artifact.source.contains(forbidden), "ordinary authority {forbidden}");
            }
            execute(case, provider, &artifact.source, rows);
        });
    }
}
fn execute(case: &str, provider: &str, text: &str, rows: usize) {
    let file = std::env::temp_dir()
        .join(format!("zryna-accounting-{}-{case}-{provider}.mjs", std::process::id()));
    std::fs::write(&file, text).expect("actual ordinary audited module bytes");
    let mut command = Command::new("node");
    command
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../zryna-backend-javascript/src/generic_execution_v1/accounting_tests/execute.mjs",
        ))
        .arg(&file)
        .arg(case);
    let outcome = process::run(command);
    let output = outcome.output;
    std::fs::write(file.with_extension("stdout"), &output.stdout).expect("retain actual stdout");
    std::fs::write(file.with_extension("stderr"), &output.stderr).expect("retain actual stderr");
    let receipt = serde_json::json!({
        "case": case, "provider": provider, "module": file,
        "status": output.status.to_string(), "exit_code": output.status.code(),
        "containment_failure": outcome.containment,
        "deadline_seconds": 60, "stream_bytes": 65536,
        "source_quota_or_semantic_result": false
    });
    std::fs::write(
        file.with_extension("status.json"),
        serde_json::to_vec_pretty(&receipt).expect("retained process receipt"),
    )
    .expect("retain status before assertions");
    assert!(
        outcome.containment.is_none(),
        "fixture host containment failure; retained {}",
        file.display()
    );
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let receipt: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("constant-space summary");
    assert_eq!(receipt["case"], case);
    assert_eq!(receipt["rows"].as_u64(), Some(u64::try_from(rows).expect("finite oracle rows")));
    assert_eq!(receipt["ordinaryPublicArtifact"], true);
    assert_eq!(receipt["pristineRetryAfterDenial"], true);
}
