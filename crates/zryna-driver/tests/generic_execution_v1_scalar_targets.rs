//! Six genuine scalar sources, both providers, all18 frozen rows on all three ordinary targets.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_scalar_targets/emit.rs"]
mod emit;
#[path = "generic_execution_v1_scalar_targets/execute.rs"]
mod execute;
#[path = "generic_execution_v1_scalar_targets/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_scalar_targets/native.rs"]
mod native;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_scalar_replay/source.rs"]
mod source;
#[test]
fn both_provider_new_seals_match_all_frozen_scalar_target_rows_and_retries() {
    let cases = [
        "wrapping-neg",
        "wrapping-sub",
        "wrapping-mul",
        "signed-relations",
        "bool-equality",
        "generic-concrete-scalars",
    ];
    let mut rows = 0;
    for case in cases {
        for provider in source::PROVIDERS {
            source::with_program(case, provider, |program, wire| {
                let directory = emit::all(case, provider, program, wire);
                rows += execute::all(case, &directory);
                println!("scalar targets {case}/{provider}: {}", directory.display());
            });
        }
    }
    assert_eq!(
        rows, 36,
        "eighteen unchanged source rows times two actual providers; each executed on three targets"
    );
}
