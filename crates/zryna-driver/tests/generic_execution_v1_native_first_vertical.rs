//! Native-only genuine new execution owner; independent of pending Wasm implementation.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_first_vertical/authority.rs"]
mod authority;
#[path = "generic_execution_v1_native_first_vertical/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_first_vertical/execute.rs"]
mod execute;
#[path = "generic_execution_v1_first_vertical/negative.rs"]
mod negative;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_first_vertical/runtime_oracle.rs"]
mod runtime_oracle;
#[path = "generic_execution_v1_first_vertical/source.rs"]
mod source;
#[test]
fn exact_frozen_originals_reach_new_native_owner_and_all_fault_retries() {
    authority::check_all();
    negative::check();
    execute::check_tools();
    let mut observations = 0;
    let mut faults = 0;
    for case in authority::POSITIVE {
        source::with_program(case, |program, wire, evidence| {
            let artifacts = emit::native(case, program, wire, evidence);
            let (count, failures) = execute::all(case, &artifacts);
            observations += count;
            faults += failures;
            println!("NEW execution native-only {case}: {}", artifacts.directory.display());
        });
    }
    assert_eq!(observations, 17);
    assert_eq!(faults, 6, "separate real cleanup-before-SIGILL fault processes");
}
