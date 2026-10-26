//! Exact source-first observations remain private test evidence, never ordinary artifact authority.
use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn actual(case: &str) {
    let bytes = include_bytes!("observed/OBSERVED_EXPECTED_FREEZE.json");
    assert_eq!(digest(bytes), "e4ffbd6b69f42d8d023fa9265b51a3bc1b08cebdc679caaa0cd3bb852032bad3");
    let expected: serde_json::Value =
        serde_json::from_slice(bytes).expect("immutable pre-observer source oracle");
    let row = expected["cases"]
        .as_array()
        .expect("twelve cases")
        .iter()
        .find(|row| row["case"] == case)
        .expect("source row");
    for provider in super::source::PROVIDERS {
        super::with_program(case, provider, |program| execute(program, case, provider, row));
    }
}
fn execute(program: &VerifiedExecution<'_>, case: &str, provider: &str, row: &serde_json::Value) {
    let artifact = super::super::emit_observed(program).expect("genuine indexed observed artifact");
    assert_eq!(
        artifact,
        super::super::emit_observed(program).expect("deterministic observed retry")
    );
    let stem = std::env::temp_dir()
        .join(format!("m416-js-indexed-observed-{}-{case}-{provider}", std::process::id()));
    let module = stem.with_extension("mjs");
    let oracle = stem.with_extension("oracle.json");
    let metadata = stem.with_extension("metadata.json");
    std::fs::write(&module, artifact.as_source()).expect("retain observed module");
    std::fs::write(&oracle, serde_json::to_vec(row).expect("oracle row"))
        .expect("retain unchanged expected row");
    let elements:Vec<_>=program.runtime().element_layouts().iter()
        .filter(|r|r.target()==StorageTarget::Linear32V1)
        .map(|r|serde_json::json!({"id":r.element().index(),"stride":r.stride(),"alignment":r.alignment()})).collect();
    std::fs::write(
        &metadata,
        serde_json::to_vec(&serde_json::json!({"elements":elements})).expect("sealed metadata"),
    )
    .expect("retain metadata");
    let mut command = Command::new("node");
    command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/indexed_tests/observed/execute.mjs"),
        )
        .arg(&module)
        .arg(&oracle)
        .arg(&metadata);
    let outcome = super::super::accounting_tests::process::run(command);
    let output = outcome.output;
    std::fs::write(stem.with_extension("stdout.json"), &output.stdout)
        .expect("retain actual trace before assertions");
    std::fs::write(stem.with_extension("stderr.log"), &output.stderr)
        .expect("retain diagnostics before assertions");
    std::fs::write(stem.with_extension("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "case":case,"provider":provider,"moduleSha256":digest(artifact.as_source().as_bytes()),
        "stdoutSha256":digest(&output.stdout),"stderrSha256":digest(&output.stderr),"status":output.status.to_string(),
        "code":output.status.code(),"containment_failure":outcome.containment,"deadline_seconds":60,"stream_bytes":65536,
        "sourceSha256":row["sourceSha256"],"private_observer_only":true,"source_quota_or_semantic_result":false
    })).expect("status metadata")).expect("retain actual status before assertions");
    assert!(outcome.containment.is_none(), "host containment; retained {stem:?}");
    assert!(
        output.status.success(),
        "{case}/{provider}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("actual trace receipt");
    assert_eq!(actual["case"], case);
    assert_eq!(actual["complete"], true);
    assert_eq!(
        actual["rows"].as_array().expect("actual observations").len(),
        row["normal"].as_array().expect("normal rows").len() * 2
            + row["faults"].as_array().expect("fault rows").len() * 2
    );
    for key in [
        "exactCanonicalEvents",
        "exactUtf8AndCloneSource",
        "exactReleaseIdentityOrder",
        "unpublishedFailureOutputUnread",
        "pristineRetries",
    ] {
        assert_eq!(actual[key], true);
    }
}
macro_rules! cases{($($name:ident,$case:literal;)*)=>{$(#[test]fn $name(){actual($case);})*};}
cases! {
    observed_array_exclusive_copy,"array-exclusive-copy";
    observed_array_indexed_rhs_match,"array-indexed-rhs-match";
    observed_array_shared_copy,"array-shared-copy";
    observed_chained_shared_copy,"chained-shared-copy";
    observed_copy_read_control,"copy-read-control";
    observed_index_once_before_rhs,"index-once-before-rhs";
    observed_vec_exclusive_replace,"vec-exclusive-replace";
    observed_vec_indexed_replace,"vec-indexed-replace";
    observed_vec_shared_clone,"vec-shared-clone";
    observed_zero_length_bounds,"zero-length-bounds";
    observed_called_indexed_shared_copy,"called-indexed-shared-copy";
    observed_indexed_push_copy,"indexed-push-copy";
}
