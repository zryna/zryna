//! Actual CFG/place/state/plan controls for M3 edge obligation renaming.
//! No `SourceProof` or public `VerifiedExecution` constructor is used.
use super::{plan, raw, typed};
mod fixture;
use fixture::{Case, input};
fn accepts(case: Case) {
    let (p, world) = input(case, false);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    plan::derive(&p, &typed, &mut 0).expect("edge renames each source obligation in place");
}
#[test]
fn early_affine_argument_keeps_its_anchor_across_a_jump() {
    accepts(Case::Single);
}
#[test]
fn edge_argument_order_does_not_reorder_independent_pending_obligations() {
    accepts(Case::Reordered);
}
#[test]
fn two_cfg_edges_preserve_the_original_parameter_acquisition_anchor() {
    accepts(Case::TwoHop);
}
#[test]
fn appended_parameter_cleanup_cannot_gain_state_plan_authority() {
    let (p, world) = input(Case::Single, true);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err(), "cleanup must retain source pending slots");
}
#[test]
fn edge_anchor_plan_budget_boundary_overflow_and_retry_are_atomic() {
    let (p, world) = input(Case::Reordered, false);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    let mut exact = 0;
    plan::derive(&p, &typed, &mut exact).expect("measure actual plan owner debit");
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut at = limit - exact;
    plan::derive(&p, &typed, &mut at).expect("exact remaining owner budget");
    assert_eq!(at, limit);
    let mut extra = limit - exact + 1;
    let before = extra;
    assert!(plan::derive(&p, &typed, &mut extra).is_err());
    assert_eq!(extra, before, "failed owner verification does not commit debit");
    let mut overflow = usize::MAX;
    assert!(plan::derive(&p, &typed, &mut overflow).is_err());
    assert_eq!(overflow, usize::MAX);
    let mut retry = 0;
    plan::derive(&p, &typed, &mut retry).expect("pristine same input retry");
    assert_eq!(retry, exact);
}
