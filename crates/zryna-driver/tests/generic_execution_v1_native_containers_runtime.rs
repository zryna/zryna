//! Four literal/new-seal/native paths,34 fixed observations,15 SIGILL faults and pristine retries.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_containers/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_containers/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_containers/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_containers/source.rs"]
mod source;
#[test]
fn genuine_container_native_runtime_observes_every_frozen_fault_and_pristine_retry() {
    let mut observations = 0;
    let mut faults = 0;
    for case in source::SETTLED {
        source::with_program(case, "native", |program, wire| {
            let artifact = emit::native(case, program, wire);
            let (count, failed) = execute::all(case, &artifact);
            observations += count;
            faults += failed;
            println!("native container {case}: {}", artifact.directory.display());
        });
    }
    assert_eq!(observations, 34);
    assert_eq!(faults, 15);
}
