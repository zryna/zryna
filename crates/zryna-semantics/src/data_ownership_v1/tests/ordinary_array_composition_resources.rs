use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::generic_vec_fixture::ordinary_array_composition_fixture::{
    Shape, fixture,
};
use crate::data_ownership_v1::type_model::Binding;
use zryna_ir::data_ownership_v1 as ir;

pub(super) fn parameters(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
    lowerer.mixed_function = true;
    for (index, parameter) in lowerer.function.parameters.iter().enumerate() {
        let ty = semantic_type(
            lowerer.file,
            parameter.type_syntax,
            lowerer.module,
            lowerer.declarations,
            lowerer.graph,
            lowerer.node_types,
            lowerer.errors,
        )
        .expect("exact parameter");
        let span = crate::data_ownership_v1::span(lowerer.input.sources(), parameter.span);
        let id = u32::try_from(index).expect("bounded parameter");
        lowerer
            .constructor_types
            .record_parameter(&raw::ValueDefinition { id: raw::ValueId(id), ty: ty.ir, span })
            .expect("dense parameter");
        let place = raw::PlaceId(id);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span,
            kind: raw::PlaceKind::Parameter(id),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
        if !ty.is_copy() {
            lowerer.owners.register_parameter(place).expect("owner");
        }
        lowerer.next_value += 1;
    }
}

fn seed(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, ty: Ty, shape: Shape) -> u32 {
    parameters(lowerer);
    let statement = if matches!(shape, Shape::Chained) {
        assert!(run_statement(lowerer, 0, ty));
        1
    } else {
        0
    };
    root_value(lowerer, statement)
}

// Values, places, transitions, cleanup plans and cleanup actions for just the
// observation, excluding the already lowered binding and subsequent return cleanup.
fn demand(owned: bool, shape: Shape) -> [usize; 5] {
    match (owned, shape) {
        (false, Shape::Fresh) => [3, 1, 6, 2, 0],
        (true, Shape::Fresh) => [3, 2, 6, 4, 4],
        (false, Shape::Chained) => [3, 0, 6, 2, 0],
        (true, Shape::Chained) => [3, 1, 6, 4, 5],
    }
}

pub(super) const LIMITS: [usize; 5] = [
    ir::MAX_VALUES_PER_FUNCTION,
    ir::MAX_PLACES_PER_FUNCTION,
    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
    ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
    ir::MAX_DROP_ACTIONS_PER_FUNCTION,
];

pub(super) fn counts(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> [usize; 5] {
    [
        lowerer.next_value as usize,
        lowerer.places.len(),
        lowerer.instructions.len(),
        lowerer.cleanup_plans.len(),
        lowerer.cleanup_actions,
    ]
}

pub(super) fn reserve(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    dimension: usize,
    held: usize,
) {
    match dimension {
        0 => lowerer.set_reserved_constructor_values_for_test(held),
        1 => lowerer.set_reserved_constructor_places_for_test(held),
        2 => lowerer.reserved_transitions = held,
        3 => lowerer.preparation_facts.held_cleanup[0] = held,
        4 => lowerer.preparation_facts.held_cleanup[1] = held,
        _ => unreachable!("resource dimension"),
    }
}

#[test]
fn ordinary_array_preparation_exact_first_extra_and_recovery_are_atomic() {
    // Authenticated small source plus synthetic external credits, not a claim
    // that this source itself contains a maximum-sized instruction arena.
    for owned in [false, true] {
        for shape in [Shape::Fresh, Shape::Chained] {
            let expected = demand(owned, shape);
            for dimension in 0..5 {
                if expected[dimension] == 0 {
                    continue;
                }
                for extra in [false, true, false] {
                    let (source, snapshot) = fixture(owned, shape, false, [2, 2], [Some(-1); 2]);
                    let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                        crate::data_ownership_v1::lower(lowerer.input)
                            .expect("full source control");
                        let value = seed(lowerer, ty, shape);
                        let initial = counts(lowerer);
                        let held = LIMITS[dimension] - initial[dimension] - expected[dimension]
                            + usize::from(extra);
                        reserve(lowerer, dimension, held);
                        let before = state(lowerer);
                        let checkpoint = lowerer.preparation_checkpoint();
                        let facts = lowerer.preparation_facts.clone();
                        if extra {
                            assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
                            assert_eq!(state(lowerer), before);
                            assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                            assert_eq!(lowerer.preparation_facts, facts);
                        } else {
                            let prepared = PreparedValue::prepare(lowerer, value, ty)
                                .expect("exact indexed preparation frontier");
                            assert_eq!(state(prepared.lowerer), before);
                            prepared.consume();
                            let actual = counts(lowerer);
                            for index in 0..5 {
                                assert_eq!(actual[index] - initial[index], expected[index]);
                            }
                            assert_eq!(actual[dimension] + held, LIMITS[dimension]);
                            assert!(lowerer.preparation_facts.active_borrows.is_empty());
                            assert!(lowerer.preparation_facts.aliases.is_empty());
                        }
                        reserve(lowerer, dimension, 0);
                        assert!(lowerer.constructor_storage_is_clear());
                    });
                    assert_eq!(errors.len(), usize::from(extra));
                    if extra {
                        assert_eq!(errors[0].code(), "ZRYNA-M3201");
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_array_preparation_active_capacity_charges_one_live_authority() {
    for owned in [false, true] {
        for shape in [Shape::Fresh, Shape::Chained] {
            for extra in [false, true, false] {
                let (source, snapshot) = fixture(owned, shape, false, [2, 2], [Some(-1); 2]);
                let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                    let value = seed(lowerer, ty, shape);
                    let count = ir::MAX_ACTIVE_BORROWS_PER_FUNCTION - 1 + usize::from(extra);
                    // Count-only formal authorities exercise preparation capacity;
                    // these are not fabricated raw parameters submitted to verification.
                    lowerer.preparation_facts.parameter_borrows = (0..count)
                        .map(|id| raw::BorrowId(u32::try_from(id).expect("bounded borrow")))
                        .collect();
                    lowerer.preparation_facts.next_borrow = u32::try_from(count).expect("count");
                    let before = state(lowerer);
                    let facts = lowerer.preparation_facts.clone();
                    if extra {
                        assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
                        assert_eq!(state(lowerer), before);
                        assert_eq!(lowerer.preparation_facts, facts);
                    } else {
                        PreparedValue::prepare(lowerer, value, ty)
                            .expect("one free authority")
                            .consume();
                        assert!(lowerer.preparation_facts.active_borrows.is_empty());
                        assert_eq!(
                            lowerer.preparation_facts.parameter_borrows,
                            facts.parameter_borrows
                        );
                        assert_eq!(
                            lowerer.preparation_facts.next_borrow - facts.next_borrow,
                            if matches!(shape, Shape::Fresh) { 1 } else { 2 }
                        );
                    }
                });
                assert_eq!(errors.len(), usize::from(extra));
                if extra {
                    assert_eq!(errors[0].code(), "ZRYNA-M3201");
                }
            }
        }
    }
}

#[test]
fn ordinary_array_preparation_borrow_identity_overflow_keeps_all_state() {
    for owned in [false, true] {
        for shape in [Shape::Fresh, Shape::Chained] {
            let (source, snapshot) = fixture(owned, shape, false, [2, 2], [Some(-1); 2]);
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                let value = seed(lowerer, ty, shape);
                // Impossible internal identity counter, distinct from source capacity.
                lowerer.preparation_facts.next_borrow = u32::MAX;
                let before = state(lowerer);
                let checkpoint = lowerer.preparation_checkpoint();
                let facts = lowerer.preparation_facts.clone();
                assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                assert_eq!(lowerer.preparation_facts, facts);
                lowerer.preparation_facts.next_borrow = 0;
                PreparedValue::prepare(lowerer, value, ty)
                    .expect("same-state valid recovery")
                    .consume();
                assert!(lowerer.preparation_facts.active_borrows.is_empty());
            });
            assert!(
                errors.is_empty(),
                "internal checked identity overflow emits no source diagnostic"
            );
        }
    }
}
