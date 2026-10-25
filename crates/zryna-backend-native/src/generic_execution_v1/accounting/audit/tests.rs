//! Mutations of actual generated SSA from genuine source/effect/NEW execution authority.
mod concat_source;
mod fixture;
mod mutations;
mod string_source;
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{FuncRef, Function, Inst, Value},
};
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
use zryna_ownership_runtime_abi::LogicalOperation as Op;
fn rejected(case: &str, operation: Op, change: impl Fn(&mut Function, Value, &[FuncRef], Inst)) {
    fixture::with_function(case, operation, |program, function, context, runtime, call| {
        assert!(
            super::function(program, function, context, runtime).is_ok(),
            "genuine pristine graph"
        );
        change(function, context, runtime, call);
        assert!(
            super::function(program, function, context, runtime).is_err(),
            "mutated actual SSA must reject"
        );
    });
}
#[test]
fn arithmetic_length_is_bound_to_each_real_canonical_operation_operand() {
    for (case, operation) in [
        ("nonempty-call-loop", Op::StringFromUtf8Copy),
        ("retained-clone-call-loop", Op::StringClone),
        ("same-source", Op::StringConcat),
    ] {
        rejected(case, operation, mutations::wrong_length);
    }
}
#[test]
fn unclassified_counter_stores_and_aliases_cannot_reset_the_invocation() {
    for mutation in [
        mutations::reset_after_call as fn(&mut Function, Value, &[FuncRef], Inst),
        mutations::denied_store,
        mutations::stray_store,
        mutations::alias_context,
    ] {
        rejected("nonempty-call-loop", Op::StringFromUtf8Copy, mutation);
    }
}
#[test]
fn output_zero_and_preflight_guard_are_real_ordered_obligations() {
    for mutation in [
        mutations::poison_output as fn(&mut Function, Value, &[FuncRef], Inst),
        mutations::missing_guard,
    ] {
        rejected("nonempty-call-loop", Op::StringFromUtf8Copy, mutation);
    }
}
#[test]
fn release_spends_the_exact_live_amount_authenticated_by_its_funding_guard() {
    rejected("nonempty-call-loop", Op::StringRelease, mutations::release_subtract_zero);
}
#[test]
fn every_private_callee_receives_the_original_context_as_its_last_argument() {
    rejected("nonempty-call-loop", Op::StringFromUtf8Copy, mutations::wrong_callee_context);
}

mod vector;
mod vector_mutations;
mod vector_source;

mod control;
mod control_mutations;
mod control_source;
mod helper_fixture;

mod source_fixture;

mod string_outcomes;

mod invocation;
