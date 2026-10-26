//! Root alone executes Node; retain actual output and module identity before assertions.
use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn execute(program: &VerifiedExecution<'_>, case: &str, provider: &str) {
    let artifact =
        super::super::emit(program).expect("genuine same-seal indexed ordinary emission");
    assert_eq!(artifact, super::super::emit(program).expect("deterministic indexed retry"));
    assert_eq!(
        digest(include_bytes!("RUNTIME_EXPECTED_FREEZE.json")),
        "d8115763d70c5f3d804f4756fb3aa6ca34da58f8f7242ebaba7c7f356a25c8d5"
    );
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("RUNTIME_EXPECTED_FREEZE.json"))
            .expect("immutable pre-output scalar and bounds oracle");
    let row = expected["cases"]
        .as_array()
        .expect("twelve rows")
        .iter()
        .find(|row| row["case"] == case)
        .expect("exact original case");
    let stem = std::env::temp_dir()
        .join(format!("m416-js-indexed-{}-{case}-{provider}", std::process::id()));
    let module = stem.with_extension("mjs");
    let oracle = stem.with_extension("oracle.json");
    std::fs::write(&module, &artifact.source).expect("retain ordinary module");
    std::fs::write(&oracle, serde_json::to_vec(row).expect("row JSON"))
        .expect("retain expected row");
    let mut command = Command::new("node");
    command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/indexed_tests/execute.mjs"),
        )
        .arg(&module)
        .arg(&oracle);
    let outcome = super::super::accounting_tests::process::run(command);
    let output = outcome.output;
    std::fs::write(stem.with_extension("stdout.json"), &output.stdout)
        .expect("retain child stdout first");
    std::fs::write(stem.with_extension("stderr.log"), &output.stderr)
        .expect("retain child stderr first");
    std::fs::write(stem.with_extension("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "case":case,"provider":provider,"success":output.status.success(),"code":output.status.code(),"status":output.status.to_string(),
        "moduleSha256":digest(artifact.source.as_bytes()),"moduleBytes":artifact.source.len(),
        "stdoutSha256":digest(&output.stdout),"stderrSha256":digest(&output.stderr),
        "sourceSha256":row["sourceSha256"],"expectedRow":row,
        "containment_failure":outcome.containment,"deadline_seconds":60,"stream_bytes":65536,
        "source_quota_or_semantic_result":false
    })).expect("actual status metadata")).expect("retain status before assertions");
    assert!(outcome.containment.is_none(), "host containment; retained {stem:?}");
    assert!(
        output.status.success(),
        "{case}/{provider}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("actual child receipt");
    assert_eq!(actual["case"], case);
    assert_eq!(actual["complete"], true);
    assert_eq!(
        actual["rows"].as_array().expect("all actual observations").len(),
        row["observations"].as_array().expect("source rows").len() * 2
    );
}

#[test]
fn indexed_host_supervision_caps_each_stream_and_reaps_before_pristine_retry() {
    for stream in ["stdout", "stderr"] {
        let mut command = Command::new("node");
        command
            .arg("-e")
            .arg(format!("process.{stream}.write('x'.repeat(65537));setInterval(()=>{{}},1000);"));
        let outcome = super::super::accounting_tests::process::run(command);
        let stem = std::env::temp_dir()
            .join(format!("m416-js-indexed-containment-{}-{stream}", std::process::id()));
        std::fs::write(stem.with_extension("stdout"), &outcome.output.stdout)
            .expect("retain bounded overflow stdout");
        std::fs::write(stem.with_extension("stderr"), &outcome.output.stderr)
            .expect("retain bounded overflow stderr");
        std::fs::write(stem.with_extension("status.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "status":outcome.output.status.to_string(),"containment_failure":outcome.containment,
            "deadline_seconds":60,"stream_bytes":65536,"source_quota_or_semantic_result":false
        })).expect("host receipt")).expect("retain containment before assertions");
        assert_eq!(outcome.containment, Some("stream"));
        assert!(outcome.output.stdout.len() <= 65537 && outcome.output.stderr.len() <= 65537);
        let mut command = Command::new("node");
        command.arg("-e").arg("process.stdout.write('pristine');");
        let retry = super::super::accounting_tests::process::run(command);
        assert!(retry.containment.is_none());
        assert!(retry.output.status.success());
        assert_eq!(retry.output.stdout, b"pristine");
        assert!(retry.output.stderr.is_empty());
    }
}
