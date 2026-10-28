//! Actual canonical native loan/replacement/failure observations, separate from nine object controls.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_loan_runtime/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_loan_runtime/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_loan_runtime/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_loans/source.rs"]
mod source;
#[test]
fn canonical_runtime_matches_frozen_native_loan_replacement_and_cleanup_observations() {
    let mut observations = 0;
    let mut faults = 0;
    for provider in source::PROVIDERS {
        for case in ["generic-replace", "static-disjoint", "fallible-cleanup"] {
            source::with_program(case, provider, |program, wire| {
                let artifact = emit::native(case, program, wire);
                let (count, failed) = execute::all(case, &artifact);
                observations += count;
                faults += failed;
                println!("native loans {case}/{provider}: {}", artifact.directory.display());
            });
        }
    }
    assert_eq!(observations, 46);
    assert_eq!(faults, 20);
}
