//! Genuine ordinary-object private control source status and exact held-frontier probes.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_control_private_accounting/binding.rs"]
mod binding;
#[path = "generic_execution_v1_native_accounting_string/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_control_private_accounting/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_accounting_string/metadata.rs"]
mod metadata;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_accounting_control/source.rs"]
mod source;
const SOURCE_ROOT: &str = "../../tests/m7-generic-execution-native-accounting-control";
const EVIDENCE_PREFIX: &str = "m416-native-control-private-accounting";
const RUNTIME_ORACLE: &[u8] =
    include_bytes!("generic_execution_v1_native_accounting_control/RUNTIME_EXPECTED_FREEZE.json");
#[test]
fn birth_source_status_zero_capacity_and_canonical_allocation_are_distinct() {
    actual("control-birth-call-loop", 4);
}
#[test]
fn denied_count_clone_returns_capacity_and_funds_original_parent_cleanup() {
    actual("control-live-count-call-loop", 2);
}
#[test]
fn known_expired_reaches_canonical_five_at_exact_held_frontier() {
    actual("control-expired-call-loop", 2);
}
fn actual(case: &str, count: usize) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifact = emit::native(case, program, wire);
            binding::write(program, &artifact.directory);
            assert_eq!(execute::all(case, &artifact), count);
            println!("native control private {case}/{provider}: {}", artifact.directory.display());
        });
    }
}
