//! Additive equivalent-Empty original and whole-source gates; original16 remain unchanged.
#[path = "generic_execution_v1_partial_canonical_empty/source.rs"]
mod source;
#[test]
fn whole_move_and_complete_field_moves_have_one_original_empty_state() {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project("whole-vs-fields", provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("whole and all-fields moves are equivalent before any closed layout");
    }
}
#[test]
fn equivalent_empty_paths_reach_the_independent_new_whole_source_issuer() {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project("whole-vs-fields", provider);
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect("original permissions precede the closed whole-source gate");
        source::with_program("whole-vs-fields", provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            assert!(program.functions().next().is_some());
        });
    }
}
