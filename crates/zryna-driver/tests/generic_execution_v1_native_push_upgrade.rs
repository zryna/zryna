//! Genuine original push/live/expired sources, checked native artifacts and actual canonical runtime.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_push_upgrade/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_push_upgrade/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_push_upgrade/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_push_upgrade/source.rs"]
mod source;
#[test]
fn both_named_provider_originals_issue_new_seals_and_checked_native_artifacts() {
    for case in source::SETTLED {
        let (_, native) = source::project(case, "native");
        let (_, typescript) = source::project(case, "typescript");
        assert_eq!(native.files(), typescript.files());
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, wire| {
                emit::native(case, program, wire);
            });
        }
    }
}
#[test]
fn native_push_and_upgrade_match_frozen_fault_cleanup_count_and_retry_observations() {
    let mut observations = 0;
    let mut faults = 0;
    for case in source::SETTLED {
        source::with_program(case, "native", |program, wire| {
            let artifact = emit::native(case, program, wire);
            let (count, failed) = execute::all(case, &artifact);
            observations += count;
            faults += failed;
            println!("native push/upgrade {case}: {}", artifact.directory.display());
        });
    }
    assert_eq!(observations, 21);
    assert_eq!(faults, 9);
}
