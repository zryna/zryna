//! Native-only actual Drop/lifecycle gates; original deep Clone remains a separate obligation.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_recursive_owner/artifact.rs"]
mod artifact;
#[path = "generic_execution_v1_native_iterative_drop/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_iterative_drop/execute.rs"]
mod execute;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_iterative_drop/source.rs"]
mod source;
use source::digest;

#[test]
fn unchanged_native_drop_sources_return_at_zero_shallow_and_deep_depths() {
    for case in ["drop-only", "controlled-cleanup", "shared-weak-recursive"] {
        source::with_program(case, |provider, program, wire| {
            let directory = emit::native(case, provider, program, wire);
            execute::ordinary(&directory);
        });
    }
}
#[test]
fn native_drop_observer_owns_lifecycle_release_order_and_controlled_cleanup() {
    for case in ["drop-only", "controlled-cleanup", "shared-weak-recursive"] {
        source::with_program(case, |provider, program, wire| {
            let directory = emit::native(case, provider, program, wire);
            execute::observed(case, &directory);
        });
    }
}
#[test]
fn genuine_recursive_entry_rejects_stale_tls_before_any_canonical_effect() {
    source::with_program("drop-only", |provider, program, wire| {
        let directory = emit::native("drop-only", provider, program, wire);
        execute::hostile(&directory);
    });
}
#[test]
fn additive_shared_weak_original_obtains_both_genuine_owners_before_runtime_claims() {
    source::with_program("shared-weak-recursive", |provider, program, wire| {
        let directory = emit::native("shared-weak-recursive", provider, program, wire);
        assert!(directory.join("program.o").is_file());
        assert_eq!(std::fs::read(directory.join("program.zir")).expect("original wire"), wire);
    });
}
