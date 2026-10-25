//! Authenticated empty Vec source, checked new artifacts and independent actual storage outputs.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_empty_vec_growth/emit.rs"]
mod emit;
#[path = "generic_execution_v1_empty_vec_growth/execute.rs"]
mod execute;
#[path = "generic_execution_v1_empty_vec_growth/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_empty_vec_growth/portable.rs"]
mod portable;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_empty_vec_growth/source.rs"]
mod source;
const CASE: &str = "empty-vec-two-pushes";
#[test]
fn both_provider_originals_issue_independent_new_whole_seals() {
    let (_, native) = source::project(CASE, "native");
    let (_, typescript) = source::project(CASE, "typescript");
    assert_eq!(native.files(), typescript.files());
    for provider in source::PROVIDERS {
        source::with_program(CASE, provider, |program, wire| {
            assert!(!wire.is_empty());
            let row = source::row(CASE);
            assert_eq!(row["pristine"]["i32"], 18);
            assert_eq!(program.scalar_abi().exports().count(), 1);
        });
    }
}
#[test]
fn native_empty_growth_observes_capacity_four_one_backing_and_atomic_fault_retry() {
    for provider in source::PROVIDERS {
        source::with_program(CASE, provider, |program, wire| {
            let artifact = emit::native(CASE, program, wire);
            execute::all(&artifact);
            println!("native empty Vec {provider}: {}", artifact.directory.display());
        });
    }
}
#[test]
fn same_new_seals_preserve_finite_javascript_and_wasm_result_and_fault_parity() {
    for provider in source::PROVIDERS {
        source::with_program(CASE, provider, |program, wire| {
            let artifact = emit::native(CASE, program, wire);
            portable::run(program, &artifact.directory);
            println!("portable empty Vec {provider}: {}", artifact.directory.display());
        });
    }
}
