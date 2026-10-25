//! Genuine original permissions and independent NEW whole seals for partial static places.
#[path = "generic_execution_v1_partial_static_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_partial_static_source_seal/source.rs"]
mod source;
fn whole(case: &str) {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("original universal partial-place permission, before closed layouts");
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert!(program.functions().next().is_some());
        });
    }
}
#[test]
fn struct_survivor() {
    whole("struct-survivor");
}
#[test]
fn struct_refill_whole() {
    whole("struct-refill-whole");
}
#[test]
fn partial_root_replacement() {
    whole("partial-root-replacement");
}
#[test]
fn nested_disjoint() {
    whole("nested-disjoint");
}
#[test]
fn array_survivor() {
    whole("array-survivor");
}
#[test]
fn array_refill() {
    whole("array-refill");
}
#[test]
fn opaque_child() {
    whole("opaque-child");
}
#[test]
fn sibling_loan() {
    whole("sibling-loan");
}
#[test]
fn failure_after_hole() {
    whole("failure-after-hole");
}
#[test]
fn branch_same_hole() {
    whole("branch-same-hole");
}
#[test]
fn loop_refill() {
    whole("loop-refill");
}
