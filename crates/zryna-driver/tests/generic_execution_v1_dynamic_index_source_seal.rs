//! Authenticated original permissions precede actual semantic/NEW source/type/state ownership.
#[path = "generic_execution_v1_dynamic_index_source_seal/checks.rs"]
mod checks;
#[path = "generic_execution_v1_dynamic_index_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
fn whole(case: &str) {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources).expect(
            "original symbolic indexed roles and universal permissions before closed types",
        );
        source::with_program(case, provider, |program, claim, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert_eq!(program.functions().len(), claim.functions.len());
            checks::claim(case, claim);
        });
    }
}
#[test]
fn copy_read_control() {
    whole("copy-read-control");
}
#[test]
fn array_shared_copy() {
    whole("array-shared-copy");
}
#[test]
fn vec_shared_clone() {
    whole("vec-shared-clone");
}
#[test]
fn array_exclusive_copy() {
    whole("array-exclusive-copy");
}
#[test]
fn vec_exclusive_replace() {
    whole("vec-exclusive-replace");
}
#[test]
fn vec_indexed_replace() {
    whole("vec-indexed-replace");
}
#[test]
fn array_indexed_rhs_match() {
    whole("array-indexed-rhs-match");
}
#[test]
fn index_once_before_rhs() {
    whole("index-once-before-rhs");
}
#[test]
fn chained_shared_copy() {
    whole("chained-shared-copy");
}
#[test]
fn zero_length_bounds() {
    whole("zero-length-bounds");
}
