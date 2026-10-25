//! Same genuine NEW execution authority reaches ordinary/observed JS, core Wasm and Native17.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_borrowed_match_targets/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_while_runtime/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_borrowed_match_targets/native.rs"]
mod native;
#[path = "generic_execution_v1_borrowed_match_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_borrowed_match_targets/oracle.rs"]
mod oracle;
#[path = "generic_execution_v1_borrowed_match_targets/portable.rs"]
mod portable;
#[path = "generic_execution_v1_partial_static_targets/portable_emit.rs"]
mod portable_emit;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_borrowed_match_source_seal/source.rs"]
mod source;
fn actual(case: &str) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifacts = emit::all(case, program, wire, true);
            portable::all(case, &artifacts);
            let (observations, faults) = native::native(case, &artifacts);
            let expected = oracle::runs(case);
            let count = expected.iter().map(|run| oracle::faults(run).len()).sum::<usize>();
            assert_eq!(faults, count);
            assert_eq!(observations, expected.len() + 2 * count);
            println!("borrowed match targets {case}/{provider}: {}", artifacts.directory.display());
        });
    }
}
macro_rules! cases { ($($name:ident => $case:literal;)*) => { $(
    #[test] fn $name() { actual($case); }
)* }; }
cases! {
    nominal_shared_copy => "nominal-shared-copy";
    nominal_shared_clone => "nominal-shared-clone";
    nominal_exclusive_copy => "nominal-exclusive-copy";
    nominal_exclusive_replace => "nominal-exclusive-replace";
    option_shared_both => "option-shared-both";
    result_shared_opaque_bound => "result-shared-opaque-bound";
    result_exclusive_both => "result-exclusive-both";
    opaque_payload_forward => "opaque-payload-forward";
    nested_exclusive_result => "nested-exclusive-result";
    branch_arm_cleanup => "branch-arm-cleanup";
    shared_reborrow_from_exclusive => "shared-reborrow-from-exclusive";
}
