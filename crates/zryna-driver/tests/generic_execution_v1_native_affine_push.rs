//! Genuine source-first affine prepared-RHS continuation, independent canonical runtime observations.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_native_affine_push/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_affine_push/execute.rs"]
mod execute;
#[path = "generic_execution_v1_native_containers/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_native_affine_push/oracle.rs"]
mod oracle;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_affine_push/source.rs"]
mod source;
#[test]
fn both_fresh_provider_originals_issue_the_whole_new_seal_and_checked_native_object() {
    let case = source::SETTLED[0];
    let (_, native) = source::project(case, "native");
    let (_, typescript) = source::project(case, "typescript");
    assert_eq!(native.files(), typescript.files());
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            emit::native(case, program, wire);
        });
    }
}
#[test]
fn native_affine_rhs_stays_owned_until_reserve_commit_and_unwinds_before_original_vector() {
    source::with_program(source::SETTLED[0], "native", |program, wire| {
        let artifact = emit::native(source::SETTLED[0], program, wire);
        let (observations, faults) = execute::all(source::SETTLED[0], &artifact);
        assert_eq!(observations, 11);
        assert_eq!(faults, 5);
        println!("native affine push: {}", artifact.directory.display());
    });
}
