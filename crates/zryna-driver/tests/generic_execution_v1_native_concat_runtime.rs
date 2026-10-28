//! Genuine concat originals execute only through the NEW issuer and canonical native runtime.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_concat_runtime/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_concat_runtime/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_concat_runtime/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_concat_runtime/source.rs"]
mod source;
#[test]
fn canonical_concat_matches_pre_output_utf8_operand_temporary_and_failure_oracles() {
    let mut observations = 0;
    let mut faults = 0;
    for provider in source::PROVIDERS {
        for case in ["local-observation", "same-source", "nested-temporaries", "formal-observation"]
        {
            source::with_program(case, provider, |program, wire| {
                let artifact = emit::native(case, program, wire);
                let (count, failed) = execute::all(case, &artifact);
                observations += count;
                faults += failed;
                println!("native concat {case}/{provider}: {}", artifact.directory.display());
            });
        }
    }
    assert_eq!(observations, 96);
    assert_eq!(faults, 44);
}
