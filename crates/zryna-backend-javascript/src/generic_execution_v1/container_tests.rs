//! Genuine whole new-domain seals and independently frozen allocation/count observations.
use std::process::Command;
pub(super) mod source;
#[test]
fn vec_copy_uses_two_actual_buffers_and_preserves_source() {
    run("vec-copy");
}
#[test]
fn vec_prepared_values_and_clone_prefix_unwind_before_outer_owners() {
    run("vec-owned-prepare");
}
#[test]
fn shared_weak_count_actions_never_clone_payload_and_free_last_control() {
    run("shared-weak-count");
}
#[test]
fn unused_opaque_count_template_does_not_acquire_runtime_resources() {
    run("opaque-count-unused");
}
fn run(case: &str) {
    assert!(source::SETTLED.contains(&case));
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program| {
            let artifact =
                super::emit_observed(program).expect("actual settled container emission");
            assert_eq!(artifact, super::emit_observed(program).expect("deterministic retry"));
            observe(case, provider, &artifact.source);
        });
    }
}
fn observe(case: &str, provider: &str, source: &str) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let file = std::env::temp_dir()
        .join(format!("zryna-container-{}-{case}-{provider}.mjs", std::process::id()));
    std::fs::write(&file, source).expect("actual emitted module");
    let output = Command::new("node")
        .arg(root.join("src/generic_execution_v1/container_tests/observe.mjs"))
        .arg(&file)
        .arg(case)
        .arg(root.join("src/generic_execution_v1/container_tests/RUNTIME_ORACLE.json"))
        .output();
    std::fs::remove_file(&file).expect("remove private artifact");
    let output = output.expect("Node observation");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    println!("{}", String::from_utf8_lossy(&output.stdout));
}
