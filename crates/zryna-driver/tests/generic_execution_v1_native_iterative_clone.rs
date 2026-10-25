//! Tests-first original recursive Clone; native heap traversal must not use host recursion.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "generic_execution_v1_recursive_owner/artifact.rs"]
mod artifact;
#[path = "generic_execution_v1_native_iterative_drop/emit.rs"]
mod emit;
#[path = "generic_execution_v1_native_iterative_clone/execute.rs"]
mod execute;
#[path = "generic_struct_v4_nested/process.rs"]
mod process;
#[path = "generic_execution_v1_native_iterative_drop/source.rs"]
mod source;
use source::digest;
fn frozen() {
    assert_eq!(
        digest(include_bytes!(
            "../../../tests/m7-generic-execution-recursive-owner/SOURCE_EXPECTED_FREEZE.json"
        )),
        "eb01f8297eb221c683d264fa35ab4f046f1a3ca3f7b561637090d77fd9b6ee24"
    );
    assert_eq!(
        digest(include_bytes!(
            "../../../tests/m7-generic-native-iterative-clone/SOURCE_RUNTIME_EXPECTED_ADDENDUM.json"
        )),
        "80c693b10cc4b66e825cf3bd67465d130adb6b34843cb6dafc71a44593ba8214"
    );
}
#[test]
fn original_native_clone_returns_twice_per_process_at_zero_shallow_and_deep_depths() {
    frozen();
    source::with_program("clone", |provider, program, wire| {
        let directory = emit::native("clone", provider, program, wire);
        execute::ordinary(&directory);
    });
}
#[test]
fn original_native_clone_leaf_failure_rolls_back_and_pristine_process_retry_matches() {
    frozen();
    source::with_program("clone", |provider, program, wire| {
        let directory = emit::native("clone", provider, program, wire);
        execute::observed(&directory);
    });
}
