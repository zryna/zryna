//! Genuine Vec3 accounting thresholds, one same-source NEW seal and canonical runtime.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_accounting_string/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_accounting_vec/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_accounting_string/metadata.rs"]
mod metadata;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_accounting_vec/source.rs"]
mod source;
const SOURCE_ROOT: &str = "../../tests/m7-generic-execution-native-accounting-vec";
const EVIDENCE_PREFIX: &str = "m416-native-accounting-vec";
const RUNTIME_ORACLE: &[u8] =
    include_bytes!("generic_execution_v1_native_accounting_vec/RUNTIME_EXPECTED_FREEZE.json");
fn actual(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifacts = emit::native(case, program, wire);
            assert_eq!(execute::all(case, &artifacts), 5);
            println!("native Vec accounting {case}/{provider}: {}", artifacts.directory.display());
        });
    }
}
#[test]
fn positive_constructor_attempt_boundary() {
    actual("nonempty-vec-call-loop");
}
#[test]
fn empty_constructor_status_and_cleanup_boundary() {
    actual("empty-vec-call-loop");
}
#[test]
fn reserve_growth_and_within_capacity_transfer_one_storage_debt() {
    actual("reserve-vec-call-loop");
}
