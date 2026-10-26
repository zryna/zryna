//! Exact native private status/effects under seeded contexts in an unchanged ordinary object.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_private_accounting/binding.rs"]
mod binding;
#[path = "generic_execution_v1_native_accounting_string/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_private_accounting/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_accounting_string/metadata.rs"]
mod metadata;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_accounting_string/source.rs"]
mod source;
fn actual(case: &str, expected: usize) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifact = emit::native(case, program, wire);
            binding::write(case, program, &artifact.directory);
            assert_eq!(execute::all(case, &artifact), expected);
            println!(
                "private native accounting {case}/{provider}: {}",
                artifact.directory.display()
            );
        });
    }
}
#[test]
fn positive_attempt_and_failed_allocation_have_exact_returned_status_and_atomic_effects() {
    actual("nonempty-call-loop", 4);
}
#[test]
fn empty_acquisition_checks_forward_plus_exact_cleanup_credit_independently_of_attempts() {
    actual("empty-call-loop", 3);
}
#[test]
fn child_capacity_preserves_real_parent_and_held_cleanup_cannot_quota_fail() {
    actual("retained-clone-call-loop", 1);
}

const SOURCE_ROOT: &str = "../../tests/m7-generic-execution-native-accounting-string";
const EVIDENCE_PREFIX: &str = "m416-native-accounting-string";
const RUNTIME_ORACLE: &[u8] =
    include_bytes!("generic_execution_v1_native_accounting_string/RUNTIME_EXPECTED_FREEZE.json");
