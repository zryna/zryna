//! Same retained v4 seal, exact nested literals, and fixed per-case allocation faults.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_struct_v4_nested/emit.rs"]
mod emit;
#[path = "generic_struct_v4_nested/execute.rs"]
mod execute;
#[path = "generic_struct_v4_nested/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_struct_v4_nested/source.rs"]
mod source;

#[test]
fn all_thirteen_complete_nested_literals_authenticate_native_and_both_frozen_providers() {
    oracle::check_frozen_authority();
    for case in source::ALL_CASES {
        let _ = source::authenticate(case);
    }
}

#[test]
fn eight_nested_originals_share_one_seal_each_across_seventy_eight_observations() {
    oracle::check_frozen_authority();
    execute::check_tools();
    let mut observations = [0; 3];
    let mut native_faults = 0;
    for case in source::POSITIVE_CASES {
        source::with_program(case, |program, wire, evidence| {
            let artifacts = emit::all_targets(case, program, wire, evidence);
            let result = execute::all_targets(case, &artifacts);
            for (total, count) in observations.iter_mut().zip(result.observations) {
                *total += count;
            }
            native_faults += result.native_faults;
            println!("Nested Struct v4 same-seal {case}: {}", artifacts.directory.display());
        });
    }
    assert_eq!(observations, [26, 26, 26]);
    assert_eq!(observations.iter().sum::<usize>(), 78);
    assert_eq!(native_faults, 9, "nine separately spawned cleanup-before-SIGILL cases");
}
