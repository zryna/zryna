//! Finite real reserved-entry statuses and cleanup; private faults grant no product options.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_invocation_traps/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_invocation_traps/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_invocation_traps/ledger.rs"]
mod ledger;
#[path = "generic_execution_v1_native_concat_runtime/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_invocation_traps/originals.rs"]
mod originals;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_invocation_traps/source.rs"]
mod source;
#[path = "generic_execution_v1_native_invocation_traps/verify.rs"]
mod verify;

#[test]
fn all_five_reserved_source_statuses_clean_owned_roots_before_process_retry() {
    let freeze = source::freeze();
    let mut children = 0;
    for provider in ["native", "typescript"] {
        for row in freeze["source_map"].as_array().expect("five frozen fault routes") {
            source::with_program(
                row["family"].as_str().expect("family"),
                provider,
                |program, wire| {
                    for retry in [false, true] {
                        emit::exercise(program, wire, row, provider, retry);
                        children += 1;
                    }
                },
            );
        }
    }
    assert_eq!(children, 20, "five faults and five pristine process retries per provider");
}
