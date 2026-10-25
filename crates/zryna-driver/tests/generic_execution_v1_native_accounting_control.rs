//! Genuine control8 accounting source thresholds through one NEW seal and canonical C17.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_accounting_string/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_accounting_control/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_accounting_string/metadata.rs"]
mod metadata;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_accounting_control/source.rs"]
mod source;
const SOURCE_ROOT: &str = "../../tests/m7-generic-execution-native-accounting-control";
const EVIDENCE_PREFIX: &str = "m416-native-accounting-control";
const RUNTIME_ORACLE: &[u8] =
    include_bytes!("generic_execution_v1_native_accounting_control/RUNTIME_EXPECTED_FREEZE.json");
fn actual(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifact = emit::native(case, program, wire);
            assert_eq!(execute::all(case, &artifact), 6);
            println!(
                "native control accounting {case}/{provider}: {}",
                artifact.directory.display()
            );
        });
    }
}
#[test]
fn control_birth_attempt_ceiling_prepays_begin_and_finish() {
    actual("control-birth-call-loop");
}
#[test]
fn all_live_count_roles_share_invocation_and_preserve_funded_parent_cleanup() {
    actual("control-live-count-call-loop");
}
#[test]
fn expired_upgrade_spends_only_forward_and_last_weak_releases_backing() {
    actual("control-expired-call-loop");
}
