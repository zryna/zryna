//! Genuine source-call accounting frontiers through the unchanged scalar native ABI.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_accounting_string/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_accounting_string/execute.rs"]
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
            assert_eq!(execute::all(case, &artifact), expected);
            println!("native accounting {case}/{provider}: {}", artifact.directory.display());
        });
    }
}
#[test]
fn positive_attempt_frontier_survives_nested_source_calls_and_resets_at_entry() {
    actual("nonempty-call-loop", 5);
}
#[test]
fn empty_string_status_frontier_has_no_physical_attempts() {
    actual("empty-call-loop", 5);
}
#[test]
fn callee_denial_cleans_retained_parent_with_held_credit() {
    actual("retained-clone-call-loop", 4);
}

const SOURCE_ROOT: &str = "../../tests/m7-generic-execution-native-accounting-string";
const EVIDENCE_PREFIX: &str = "m416-native-accounting-string";
const RUNTIME_ORACLE: &[u8] =
    include_bytes!("generic_execution_v1_native_accounting_string/RUNTIME_EXPECTED_FREEZE.json");
