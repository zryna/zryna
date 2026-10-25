//! Genuine missing scalar source forms and independent opaque-bound controls; tests FIRST.
#[path = "generic_execution_v1_scalar_replay/negative.rs"]
mod negative;
#[path = "generic_execution_v1_scalar_replay/positive.rs"]
mod positive;
#[path = "generic_execution_v1_scalar_replay/source.rs"]
mod source;
#[test]
fn wrapping_negation_source_reaches_new_whole_issuer() {
    positive::check("wrapping-neg");
}
#[test]
fn wrapping_subtraction_source_reaches_new_whole_issuer() {
    positive::check("wrapping-sub");
}
#[test]
fn wrapping_multiplication_source_reaches_new_whole_issuer() {
    positive::check("wrapping-mul");
}
#[test]
fn signed_relations_source_reaches_new_whole_issuer() {
    positive::check("signed-relations");
}
#[test]
fn bool_equality_source_reaches_new_whole_issuer() {
    positive::check("bool-equality");
}
#[test]
fn generic_concrete_scalars_source_reaches_new_whole_issuer() {
    positive::check("generic-concrete-scalars");
}
#[test]
fn opaque_negation_preserves_exact_owning_m7002() {
    negative::check("opaque-negation");
}
#[test]
fn opaque_inequality_preserves_exact_owning_m7002() {
    negative::check("opaque-inequality");
}
