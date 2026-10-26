//! Actual generated source/helper control graphs; raw counters cannot mint effects.
use super::*;
fn rejected(case: &str, operation: Op, change: impl Fn(&mut Function, Value, &[FuncRef], Inst)) {
    fixture::with_function(case, operation, |p, f, context, runtime, call| {
        assert!(
            super::super::function(p, f, context, runtime).is_ok(),
            "genuine pristine control graph"
        );
        change(f, context, runtime, call);
        assert!(
            super::super::function(p, f, context, runtime).is_err(),
            "actual control SSA mutation must reject"
        );
    });
}
#[test]
fn birth_geometry_two_held_and_no_debit_denial_are_independent_obligations() {
    for mutation in [
        control_mutations::wrong_size as fn(&mut Function, Value, &[FuncRef], Inst),
        control_mutations::hold_zero,
        control_mutations::narrow_zero,
        mutations::reset_after_call,
        mutations::denied_store,
        mutations::alias_context,
        mutations::poison_output,
    ] {
        rejected("control-birth-call-loop", Op::Allocate, mutation);
    }
}
#[test]
fn real_count_operands_known_failures_and_canonical_status_sets_are_bound() {
    for op in [Op::StrongClone, Op::WeakDowngrade, Op::WeakClone, Op::WeakUpgrade] {
        rejected("control-live-count-call-loop", op, control_mutations::wrong_count);
    }
    for op in [Op::StrongClone, Op::WeakDowngrade, Op::WeakClone, Op::WeakUpgrade] {
        rejected("control-live-count-call-loop", op, control_mutations::wrong_maximum);
    }
    rejected(
        "control-live-count-call-loop",
        Op::StrongClone,
        control_mutations::outside_guard_count,
    );
    rejected(
        "control-expired-call-loop",
        Op::WeakUpgrade,
        control_mutations::upgrade_without_alive,
    );
    rejected("control-expired-call-loop", Op::WeakUpgrade, control_mutations::expired_as_capacity);
}
#[test]
fn actual_begin_finish_and_weak_release_authenticate_output_and_live_predicates() {
    rejected(
        "control-live-count-call-loop",
        Op::StrongReleaseBegin,
        control_mutations::begin_decrements_live,
    );
    rejected(
        "control-live-count-call-loop",
        Op::StrongReleaseFinish,
        control_mutations::finish_reads_strong,
    );
    rejected(
        "control-live-count-call-loop",
        Op::StrongReleaseFinish,
        control_mutations::finish_reads_after_free,
    );
    rejected("control-expired-call-loop", Op::WeakRelease, control_mutations::missing_out_check);
    rejected("control-expired-call-loop", Op::WeakRelease, mutations::release_subtract_zero);
    for op in [Op::StrongReleaseBegin, Op::WeakRelease] {
        rejected("control-live-count-call-loop", op, control_mutations::zero_after_call);
    }
}
fn stale(offset: i32) {
    rejected("same-source", Op::StringConcat, |f, context, runtime, call| {
        control_mutations::stale_counter(f, context, runtime, call, offset);
    });
}
#[test]
fn current_counter_authority_a_rejects_prior_event_snapshot() {
    stale(0);
}
#[test]
fn current_counter_authority_l_rejects_prior_event_snapshot() {
    stale(8);
}
#[test]
fn current_counter_authority_s_rejects_prior_event_snapshot() {
    stale(16);
}
#[test]
fn current_counter_authority_h_rejects_prior_event_snapshot() {
    stale(24);
}
#[test]
fn shared_string_guard_requires_a_full_width_zero() {
    rejected("same-source", Op::StringConcat, control_mutations::narrow_zero);
}
