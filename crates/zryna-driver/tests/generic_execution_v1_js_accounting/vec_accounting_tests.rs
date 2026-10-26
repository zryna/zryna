//! Exact moved ordinary Vec source/call frontiers and their unchanged oracle.
#[path = "../generic_execution_v1_native_accounting_vec/source.rs"]
mod source;
use sha2::{Digest, Sha256};
use std::process::Command;
#[test]
fn positive_vec_attempt_frontier_preserves_same_module_reset_and_denial_retry() {
    actual("nonempty-vec-call-loop");
}
#[test]
fn empty_vec_owner_keeps_release_credit_at_the_exact_status_frontier() {
    actual("empty-vec-call-loop");
}
#[test]
fn first_growth_and_within_capacity_reserve_share_one_invocation_context() {
    actual("reserve-vec-call-loop");
}
fn actual(case: &str) {
    let oracle = include_bytes!(
        "../../../zryna-backend-javascript/src/generic_execution_v1/vec_accounting_tests/RUNTIME_EXPECTED_FREEZE.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(oracle)),
        "74e37c21295455022586c9115c6c1ed3b9e6975c1864304241660d5b55949a64"
    );
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert!(program.operations().any(|op| op.vec_construct().is_some()));
            let artifact = super::emit(program).expect("ordinary genuine NEW Vec cycle");
            assert_eq!(
                artifact,
                super::emit(program).expect("deterministic audited ordinary bytes")
            );
            for forbidden in [
                "globalThis",
                "Symbol.for",
                "import ",
                "import(",
                "require(",
                "eval(",
                "new Function",
            ] {
                assert!(
                    !artifact.source.contains(forbidden),
                    "ordinary private authority {forbidden}"
                );
            }
            execute(case, provider, &artifact.source);
        });
    }
}
fn execute(case: &str, provider: &str, text: &str) {
    let file = std::env::temp_dir()
        .join(format!("zryna-vec-accounting-{}-{case}-{provider}.mjs", std::process::id()));
    std::fs::write(&file, text).expect("retain actual audited Vec artifact");
    let mut command = Command::new("node");
    command
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../zryna-backend-javascript/src/generic_execution_v1/vec_accounting_tests/execute.mjs",
        ))
        .arg(&file)
        .arg(case);
    let outcome = super::accounting_tests::process::run(command);
    let output = outcome.output;
    std::fs::write(file.with_extension("stdout"), &output.stdout).expect("retain output");
    std::fs::write(file.with_extension("stderr"), &output.stderr).expect("retain diagnostics");
    let receipt = serde_json::json!({"case":case,"provider":provider,"module":file,
        "status":output.status.to_string(),"exit_code":output.status.code(),
        "containment_failure":outcome.containment,"deadline_seconds":60,"stream_bytes":65536,
        "source_quota_or_semantic_result":false});
    std::fs::write(
        file.with_extension("status.json"),
        serde_json::to_vec_pretty(&receipt).expect("bounded receipt"),
    )
    .expect("retain before assertion");
    assert!(outcome.containment.is_none(), "host containment; retained {}", file.display());
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("constant-space outcome summary");
    assert_eq!(summary["case"], case);
    assert_eq!(summary["rows"], 5);
    assert_eq!(summary["invocations"], 6);
    assert_eq!(summary["denials"], 1);
    assert_eq!(summary["ordinaryPublicArtifact"], true);
    assert_eq!(summary["pristineRetryAfterDenial"], true);
}
