//! Exact loop ownership receipts from real private CFG/place/state kernels.
//! These controls do not bypass source checking or issue public execution authority.
use super::{plan, raw, typed};
pub(super) mod fixture;
use fixture::input;
fn derive(p: &raw::Program, w: &crate::generic_v1::execution_v1::type_world::TypeWorld) -> bool {
    let checked = typed::container_test_support::phase_inputs(p, w);
    plan::derive(p, &checked, &mut 0).is_ok()
}
#[test]
fn each_iteration_releases_body_owner_and_preserves_outer_failure_root() {
    let (p, w) = input(false);
    assert!(derive(&p, &w), "body initialization/drop restores exact header state");
    let mut missing = p.clone();
    let _ = missing.functions[0].blocks[2].instructions.pop();
    assert!(!derive(&missing, &w), "body owner cannot disappear at the latch");
    let mut outer = p.clone();
    outer.functions[0].blocks[2].instructions[1].operation = raw::Operation::DropPlace { place: 0 };
    assert!(!derive(&outer, &w), "outer owner must survive every backedge");
    let mut cleanup = p.clone();
    cleanup.functions[0].cleanup_plans[0].actions.clear();
    assert!(!derive(&cleanup, &w), "failure must retain the original live outer owner");
    assert!(derive(&p, &w), "no failed replay poisons the pristine loop");
}
#[test]
fn lexical_body_loan_ends_before_backedge_and_owner_release() {
    let (p, w) = input(true);
    assert!(derive(&p, &w), "body lexical authority ends before the latch");
    let mut hostile = p.clone();
    hostile.functions[0].blocks[2].instructions.remove(2);
    assert!(!derive(&hostile, &w), "a live body loan cannot cross the backedge");
    assert!(derive(&p, &w), "pristine retry after missing lexical end");
}
#[test]
fn loop_plan_exact_remaining_extra_overflow_and_retry_are_atomic() {
    let (p, w) = input(true);
    let checked = typed::container_test_support::phase_inputs(&p, &w);
    let mut cost = 0;
    plan::derive(&p, &checked, &mut cost).expect("actual finite per-block/edge replay");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    plan::derive(&p, &checked, &mut exact).expect("exact remaining owner ledger");
    assert_eq!(exact, limit);
    for mut short in [limit - cost + 1, usize::MAX] {
        let before = short;
        assert!(plan::derive(&p, &checked, &mut short).is_err());
        assert_eq!(short, before, "no partial state/plan/debit publication");
    }
    let mut retry = 0;
    plan::derive(&p, &checked, &mut retry).expect("same pristine loop and ledger");
    assert_eq!(retry, cost);
}
