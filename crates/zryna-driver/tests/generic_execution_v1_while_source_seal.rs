//! Genuine provider source to the independently issued NEW whole execution authority.
#[path = "generic_execution_v1_while_source_seal/negative.rs"]
mod negative;
#[path = "generic_execution_v1_while_source_seal/source.rs"]
mod source;
use zryna_ir::generic_v1::execution_v1::raw;
fn whole(case: &str, has_loop: bool) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            if has_loop {
                assert!(program.functions().any(|function| {
                    function.blocks().any(|block| {
                        matches!(block.terminator(), raw::Terminator::Jump(edge) if edge.target <= block.id())
                    })
                }), "the genuine source loop retains a real backedge");
            }
        });
    }
}
#[test]
fn ordinary_copy_assignment_preserves_mutability_and_physical_reads() {
    whole("copy-assignment", false);
}
#[test]
fn zero_and_multiple_iterations_keep_the_same_header_state() {
    whole("zero-multiple", true);
}
#[test]
fn nested_loop_body_locals_end_before_each_latch() {
    whole("nested-lexical", true);
}
#[test]
fn early_return_cleans_body_before_enclosing_owners() {
    whole("early-return", true);
}
#[test]
fn body_lexical_loan_and_clone_end_before_repeated_entry() {
    whole("body-loan-cleanup", true);
}
#[test]
fn opaque_owned_parameter_is_retained_without_copy_or_clone_assumptions() {
    whole("opaque-retained", true);
}
#[test]
fn ordinary_owned_replacement_keeps_rhs_before_old_drop_and_pending_anchor() {
    whole("owned-root-replacement", false);
}
#[test]
fn branch_mutable_copy_updates_survive_the_shared_join() {
    whole("branch-copy-update", false);
}
#[test]
fn match_condition_repeats_and_cleans_temporary_storage_before_backedge() {
    whole("match-condition", true);
}
