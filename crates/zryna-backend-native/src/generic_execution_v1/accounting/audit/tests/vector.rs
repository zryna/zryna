//! Mutate only actual generated Vec graphs from fixed genuine authenticated source.
use super::*;
fn rejected(case: &str, operation: Op, change: impl Fn(&mut Function, Value, &[FuncRef], Inst)) {
    fixture::with_function(case, operation, |program, function, context, runtime, call| {
        assert!(
            super::super::function(program, function, context, runtime).is_ok(),
            "genuine pristine Vec graph"
        );
        change(function, context, runtime, call);
        assert!(
            super::super::function(program, function, context, runtime).is_err(),
            "mutated actual Vec SSA must reject"
        );
    });
}
#[test]
fn actual_vec_allocation_request_and_denial_counter_writes_are_bound() {
    rejected("nonempty-vec-call-loop", Op::VecAllocate, vector_mutations::wrong_request);
    for mutation in [
        mutations::reset_after_call as fn(&mut Function, Value, &[FuncRef], Inst),
        mutations::denied_store,
        mutations::alias_context,
        mutations::poison_output,
        mutations::missing_guard,
    ] {
        rejected("nonempty-vec-call-loop", Op::VecAllocate, mutation);
    }
}
#[test]
fn canonical_growth_classifier_retirement_and_status_traps_are_bound() {
    for mutation in [
        vector_mutations::wrong_request as fn(&mut Function, Value, &[FuncRef], Inst),
        vector_mutations::wrong_doubling,
        vector_mutations::retire_zero,
        vector_mutations::missing_allowed_status,
        vector_mutations::missing_impossible_status,
    ] {
        rejected("reserve-vec-call-loop", Op::VecReserve, mutation);
    }
}
#[test]
fn actual_storage_release_uses_capacity_even_when_length_is_zero() {
    rejected("empty-vec-call-loop", Op::VecReleaseStorage, vector_mutations::release_length);
    rejected("nonempty-vec-call-loop", Op::VecReleaseStorage, mutations::release_subtract_zero);
}
