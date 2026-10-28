//! Finite genuine While/assignment observations through NEW authority and canonical native runtime.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_while_runtime/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_while_runtime/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_while_runtime/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_while_runtime/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_while_source_seal/source.rs"]
mod source;
#[test]
fn finite_while_and_assignment_inputs_preserve_dynamic_cleanup_and_canonical_faults() {
    let mut observations = 0;
    let mut faults = 0;
    for provider in source::PROVIDERS {
        for case in [
            "copy-assignment",
            "zero-multiple",
            "nested-lexical",
            "early-return",
            "body-loan-cleanup",
            "opaque-retained",
            "owned-root-replacement",
            "branch-copy-update",
            "match-condition",
        ] {
            source::with_program(case, provider, |program, wire| {
                let artifact = emit::native(case, program, wire);
                let (count, failed) = execute::all(case, &artifact);
                observations += count;
                faults += failed;
                println!("native While {case}/{provider}: {}", artifact.directory.display());
            });
        }
    }
    assert_eq!(observations, 132);
    assert_eq!(faults, 52);
}
