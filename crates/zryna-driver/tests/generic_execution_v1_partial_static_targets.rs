//! Source-bound static partial ownership reaches the same verified execution seal's three actual target consumers.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_partial_static_targets/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_while_runtime/metadata.rs"]
mod metadata;
#[path = "generic_execution_v1_partial_static_targets/native.rs"]
mod native;
#[path = "generic_execution_v1_partial_static_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_partial_static_targets/oracle.rs"]
mod oracle;
#[path = "generic_execution_v1_partial_static_targets/portable.rs"]
mod portable;
#[path = "generic_execution_v1_partial_static_targets/portable_emit.rs"]
mod portable_emit;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_partial_static_targets/source.rs"]
mod source;
fn actual(case: &str) {
    assert!(source::CASES.contains(&case));
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            let artifacts = emit::all(case, program, wire);
            portable::all(case, &artifacts);
            let (observations, faults) = native::native(case, &artifacts);
            let expected = oracle::runs(case);
            let count = expected.iter().map(|run| oracle::faults(run).len()).sum::<usize>();
            assert_eq!(faults, count);
            assert_eq!(observations, expected.len() + 2 * count);
            println!("partial targets {case}/{provider}: {}", artifacts.directory.display());
        });
    }
}
macro_rules! cases { ($($name:ident => $case:literal;)*) => { $(
    #[test] fn $name() { actual($case); }
)* }; }
cases! {
    struct_survivor => "struct-survivor";
    struct_refill_whole => "struct-refill-whole";
    partial_root_replacement => "partial-root-replacement";
    nested_disjoint => "nested-disjoint";
    array_survivor => "array-survivor";
    array_refill => "array-refill";
    opaque_child => "opaque-child";
    sibling_loan => "sibling-loan";
    failure_after_hole => "failure-after-hole";
    branch_same_hole => "branch-same-hole";
    loop_refill => "loop-refill";
    canonical_empty => "whole-vs-fields";
}
