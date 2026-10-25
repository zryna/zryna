//! Genuine mixed affine/Copy storage reaches the independent whole-source execution issuer.
#[path = "generic_execution_v1_partial_mixed_copy/source.rs"]
mod source;
#[test]
fn moving_the_last_affine_child_preserves_initialized_copy_sibling_storage() {
    genuine_whole_source("move-text-read-copy");
}
#[test]
fn refilling_the_affine_child_restores_whole_move_with_copy_sibling_storage() {
    genuine_whole_source("refill-text-whole-move");
}
fn genuine_whole_source(case: &str) {
    assert_eq!(source::row(case)["expect"], "whole-seal");
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(case, provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("universal original ownership precedes the closed whole-source gate");
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert!(program.functions().next().is_some());
        });
    }
}
