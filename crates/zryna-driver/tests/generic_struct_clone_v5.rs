//! Seven exact originals, one retained v5 issuer seal each,147 target observations.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_struct_clone_v5/emit.rs"]
mod emit;
#[path = "generic_struct_clone_v5/execute.rs"]
mod execute;
#[path = "generic_struct_clone_v5/native_identity.rs"]
mod native_identity;
#[path = "generic_struct_clone_v5/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_struct_clone_v5/source.rs"]
mod source;

#[test]
fn seven_struct_clone_originals_share_one_v5_seal_each_across_147_observations() {
    oracle::check_frozen_authority();
    execute::check_tools();
    let mut observations = [0; 3];
    let mut native_faults = 0;
    for case in source::POSITIVE_CASES {
        source::with_program(case, |program, wire, evidence| {
            let artifacts = emit::all_targets(case, program, wire, evidence);
            let counts = execute::all_targets(case, &artifacts);
            for (total, count) in observations.iter_mut().zip(counts.observations) {
                *total += count;
            }
            native_faults += counts.native_faults;
            println!("Struct Clone v5 same-seal {case}: {}", artifacts.directory.display());
        });
    }
    assert_eq!(observations, [49, 49, 49]);
    assert_eq!(observations.iter().sum::<usize>(), 147);
    assert_eq!(native_faults, 21, "separately spawned exact cleanup-before-SIGILL faults");
}
