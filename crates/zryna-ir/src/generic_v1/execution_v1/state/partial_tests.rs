//! Actual new-domain state/plan owner gates; no source proof or execution seal is fabricated.
use super::{plan, raw, typed};
mod fixture;
use fixture::{Case, input};

#[test]
fn complete_struct_and_array_subobjects_move_then_cleanup_only_remaining_children() {
    for case in [Case::Struct, Case::Array, Case::OutOfOrder, Case::NestedEnum] {
        let (p, world) = input(case, false, false);
        let typed = typed::container_test_support::phase_inputs(&p, &world);
        plan::derive(&p, &typed, &mut 0).expect("complete source-world static subtree move");
    }
}
#[test]
fn exact_field_reinitialization_restores_whole_owner_and_partial_root_drop_is_legal() {
    for reinitialize in [false, true] {
        let (p, world) = input(Case::Struct, reinitialize, !reinitialize);
        let typed = typed::container_test_support::phase_inputs(&p, &world);
        plan::derive(&p, &typed, &mut 0).expect("same-owner reinit or exact remaining-child drop");
    }
}
#[test]
fn missing_declared_sibling_never_authorizes_partial_root_cleanup() {
    let (mut p, world) = input(Case::Struct, false, false);
    // Remove the sibling while retaining dense IDs and the actual selected source type.
    p.functions[0].places.remove(2);
    p.functions[0].places[2].id = 2;
    p.functions[0].cleanup_plans[0].actions = vec![raw::CleanupAction::DropPlace(0)];
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err());
}
#[test]
fn duplicate_path_alias_cannot_resurrect_or_move_the_same_owned_field_twice() {
    let (mut p, world) = input(Case::Struct, false, false);
    let f = &mut p.functions[0];
    f.places.push(raw::Place {
        id: 4,
        ty: f.places[1].ty,
        span: f.original_span,
        kind: raw::PlaceKind::StructField { base: 0, ordinal: 0 },
    });
    f.places.push(raw::Place {
        id: 5,
        ty: f.places[1].ty,
        span: f.original_span,
        kind: raw::PlaceKind::Temporary(2),
    });
    let mut second = f.blocks[0].instructions[0].clone();
    second.result.as_mut().expect("String result").id = 2;
    second.operation = raw::Operation::ReadPlace { place: 4, mode: raw::ReadMode::Move };
    f.blocks[0].instructions.push(second);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err());
}
#[test]
fn partial_parent_never_becomes_a_complete_affine_return_value() {
    let (mut p, world) = input(Case::Struct, false, false);
    let raw::Parameter::Value(parameter) = &p.functions[0].parameters[0] else { unreachable!() };
    p.functions[0].result_type = parameter.ty;
    p.functions[0].blocks[0].terminator = raw::Terminator::Return { value: Some(0), cleanup: 0 };
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err());
}

#[test]
fn partial_drop_receipt_is_exact_and_actual_owner_budget_retries_are_pristine() {
    let (p, world) = input(Case::Struct, false, true);
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    let mut cost = 0;
    let baseline = plan::derive(&p, &typed, &mut cost).expect("actual retained partial drop owner");
    let operation = typed.functions[0].sites[0][1];
    let range = baseline.operations[operation].drops.expect("old-drop receipt");
    assert!(!range.whole);
    assert_eq!(&baseline.drop_targets[range.start..range.end], &[raw::PlaceOrLoan::Place(2)]);
    let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
    let mut exact = limit - cost;
    plan::derive(&p, &typed, &mut exact).expect("exact remaining real replay credit");
    assert_eq!(exact, limit);
    for mut short in [limit - cost + 1, usize::MAX] {
        let before = short;
        assert!(plan::derive(&p, &typed, &mut short).is_err());
        assert_eq!(short, before, "no partial plan/work publication");
    }
    let retry = plan::derive(&p, &typed, &mut 0).expect("pristine owning retry");
    assert_eq!(retry.drop_targets, baseline.drop_targets);
}
#[test]
fn missing_array_elements_do_not_turn_claimed_rows_into_a_complete_array() {
    let (mut p, world) = input(Case::Array, false, false);
    p.functions[0].places.remove(2);
    p.functions[0].places[2].id = 2;
    p.functions[0].cleanup_plans[0].actions = vec![raw::CleanupAction::DropPlace(0)];
    let typed = typed::container_test_support::phase_inputs(&p, &world);
    assert!(plan::derive(&p, &typed, &mut 0).is_err());
}
