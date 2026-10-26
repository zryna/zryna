//! Windows owns the same finite deep JS/core-Wasm acceptance without importing Native/Unix code.
#![cfg(windows)]
#[path = "generic_execution_v1_recursive_portable_acceptance/execute.rs"]
mod execute;
#[path = "generic_execution_v1_recursive_portable_acceptance/process.rs"]
mod process;
#[path = "generic_execution_v1_recursive_portable_acceptance/retained.rs"]
mod retained;
#[path = "generic_execution_v1_recursive_portable_acceptance/source.rs"]
mod source;
use retained::retain;
#[test]
fn recursive_drop_original_source_and_same_seal() {
    source::actual("drop-only");
}
#[test]
fn recursive_clone_original_source_and_same_seal() {
    source::actual("clone");
}
#[test]
fn recursive_controlled_cleanup_original_source_and_same_seal() {
    source::actual("controlled-cleanup");
}
