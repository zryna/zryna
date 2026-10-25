//! Five exact originals, one NEW seal each,51 fixed runtime observations +opaque source gate.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_first_vertical/authority.rs"]
mod authority;
#[path = "generic_execution_v1_first_vertical/emit.rs"]
mod emit;
#[path = "generic_execution_v1_first_vertical/execute.rs"]
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
fn exact_originals_share_one_new_seal_each_across_all_three_target_observations() {
    authority::check_all();
    negative::check();
    execute::check_tools();
    let mut observations = [0; 3];
    let mut native_faults = 0;
    for case in authority::POSITIVE {
        source::with_program(case, |program, wire, evidence| {
            let artifacts = emit::all(case, program, wire, evidence);
            let counts = execute::all(case, &artifacts);
            for (total, count) in observations.iter_mut().zip(counts.observations) {
                *total += count;
            }
            native_faults += counts.native_faults;
            println!("NEW execution same-seal {case}: {}", artifacts.directory.display());
        });
    }
    assert_eq!(observations, [17, 17, 17]);
    assert_eq!(observations.iter().sum::<usize>(), 51);
    assert_eq!(native_faults, 6, "separate exact cleanup-before-SIGILL faults");
}
