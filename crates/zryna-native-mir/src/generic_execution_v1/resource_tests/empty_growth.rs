//! Actual two-push genuine owner preserves native exact/extra/overflow and fresh retry gates.
#[path = "../../../../zryna-driver/tests/generic_execution_v1_empty_vec_growth/source.rs"]
mod source;
#[test]
fn genuine_empty_then_two_pushes_reach_native_resource_boundary() {
    for provider in source::PROVIDERS {
        source::with_program("empty-vec-two-pushes", provider, |program, _| {
            assert_eq!(program.operations().filter(|op| op.vec_push().is_some()).count(), 2);
            super::check_boundary(program, "empty-vec-two-pushes");
        });
    }
}
