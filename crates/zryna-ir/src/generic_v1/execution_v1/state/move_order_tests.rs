//! Normative M3 pending-obligation order, exercised at actual state/plan owner gates.
//! No `SourceProof` or `VerifiedExecution` is created by these phase-local controls.
use super::{plan, raw, typed};
mod fixture;
use fixture::{Case, input};
fn accepts(case: Case) {
    let (p, world) = input(case, false);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    plan::derive(&p, &typed, &mut 0).expect("M3 pending obligation is renamed at its source slot");
}
#[test]
fn whole_move_to_ssa_retains_its_position_before_later_affine_parameters() {
    accepts(Case::ReadMove);
}
#[test]
fn whole_move_then_later_local_install_preserves_the_original_pending_anchor() {
    accepts(Case::ReadThenInstall);
}
#[test]
fn fresh_whole_install_renames_an_earlier_parameter_without_appending_it() {
    accepts(Case::FreshInstall);
}
#[test]
fn whole_root_replacement_uses_rhs_anchor_after_removing_old_target_in_both_orders() {
    accepts(Case::ReplaceEarly);
    accepts(Case::ReplaceLate);
}
#[test]
fn append_order_claim_cannot_replace_normative_pending_order() {
    let (p, world) = input(Case::FreshInstall, true);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err(), "wrong owner cleanup cannot gain authority");
}
