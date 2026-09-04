use super::*;

fn state_counts(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> [usize; 6] {
    [
        lowerer.instructions.len(),
        lowerer.places.len(),
        lowerer.cleanup_plans.len(),
        lowerer.cleanup_actions,
        lowerer.owners.pending().len(),
        lowerer.next_value as usize,
    ]
}

#[test]
fn constructor_envelope_whole_clone_rejects_held_capacity_before_cleanup_mutation() {
    for dimension in 1..4 {
        let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let mut held = [0; 4];
            let current = [
                lowerer.aggregate_operands,
                lowerer.instructions.len(),
                lowerer.next_value as usize,
                lowerer.places.len(),
            ];
            held[dimension] = LIMITS[dimension] - current[dimension] - 1;
            set_credits(lowerer, held);
            let expression = root_value(lowerer, 1);
            let at = span(
                lowerer.input.sources(),
                lowerer.function.body.expressions[expression as usize].span,
            );
            let parent = lowerer.reserve_constructor_commit(result, 2, at).expect("parent fits");
            let before = state_counts(lowerer);
            assert!(lowerer.value(expression, result).is_none());
            assert_eq!(state_counts(lowerer), before, "clone rejected before cleanup/emission");
            parent.release(lowerer);
            assert_eq!(credits(lowerer), held);
        });
        assert_diagnostic(
            &errors,
            "structural clone exceeds a checked value, place, or cleanup resource limit",
        );
    }
}

#[test]
fn constructor_envelope_projection_creation_respects_parent_place_credit_and_cache_reuse() {
    let errors = with_fixture(Fixture::Projection, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let held = [0, 0, 0, LIMITS[3] - lowerer.places.len() - 1];
        set_credits(lowerer, held);
        let before = state_counts(lowerer);
        assert!(lowerer.value(root_value(lowerer, 1), result).is_none());
        assert_eq!(state_counts(lowerer), before);
        assert_eq!(credits(lowerer), held);
        assert!(lowerer.projections.is_empty());
    });
    assert_diagnostic(&errors, "derived owned projection places exceed the per-function M3 limit");
    let errors = with_fixture(Fixture::Projection, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let expression = lowerer
            .function
            .body
            .expressions
            .iter()
            .enumerate()
            .find(|(_, expression)| {
                matches!(&expression.kind,
                RawExpressionKind::FieldAccess { field, .. } if field.text == "first")
            })
            .map(|(index, _)| u32::try_from(index).expect("projection expression"))
            .expect("real field projection");
        let projection = lowerer.owned_place(expression).expect("new projection");
        let before = lowerer.places.len();
        lowerer.constructor_storage.places = LIMITS[3] - before;
        let repeated = lowerer.owned_place(expression).expect("cached projection needs no credit");
        assert_eq!(projection.place, repeated.place);
        assert_eq!(lowerer.places.len(), before);
    });
    assert!(errors.is_empty());
}

#[test]
fn constructor_envelope_direct_partial_transfer_cannot_consume_parent_value_credit() {
    let errors = with_fixture(Fixture::PartialTransfer, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        assert!(run_statement(lowerer, 1, result));
        assert!(!lowerer.partial_roots.is_empty(), "genuine partial source before transfer");
        let held = [0, 0, LIMITS[2] - lowerer.next_value as usize - 1, 0];
        set_credits(lowerer, held);
        let at = span(lowerer.input.sources(), lowerer.function.body.statements[2].span);
        let parent = lowerer.reserve_constructor_commit(result, 2, at).expect("parent fits");
        let before = state_counts(lowerer);
        let pending = lowerer.owners.pending().to_vec();
        let masks = lowerer.moved_projections.clone();
        assert!(!run_statement(lowerer, 2, result));
        assert_eq!(state_counts(lowerer), before);
        assert_eq!(lowerer.owners.pending(), pending);
        assert_eq!(lowerer.moved_projections, masks);
        parent.release(lowerer);
        assert_eq!(credits(lowerer), held);
    });
    assert_diagnostic(&errors, "partial aggregate transfer exceeds the per-function value limit");
}

#[test]
fn constructor_envelope_keeps_existing_assignment_transition_credit_exactly_once() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        let at = span(lowerer.input.sources(), lowerer.function.span);
        assert!(lowerer.reserve_transition(at));
        assert_eq!(lowerer.reserved_transitions, 1);
        let before = credits(lowerer);
        lowerer.value(root_value(lowerer, 0), result).expect("constructor under assignment credit");
        assert_eq!(credits(lowerer), before);
        assert_eq!(lowerer.instructions.len(), 3);
        lowerer.release_transition();
        assert_eq!(credits(lowerer), [0; 4]);
    });
    assert!(errors.is_empty());
}

#[test]
fn constructor_envelope_zero_cost_dimensions_still_reject_overflowed_held_usage() {
    for dimension in [0, 3] {
        let errors = with_fixture(Fixture::Pair, |lowerer, _| {
            let copy = lowerer
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == zryna_layout::TypeCategory::Bool)
                .copied()
                .expect("Copy type");
            let at = span(lowerer.input.sources(), lowerer.function.span);
            let mut held = [0; 4];
            held[dimension] = LIMITS[dimension];
            set_credits(lowerer, held);
            let ticket = lowerer
                .reserve_constructor_commit(copy, 0, at)
                .expect("zero additional cost at exact limit");
            ticket.release(lowerer);
            assert_eq!(credits(lowerer), held);
            held[dimension] = usize::MAX;
            set_credits(lowerer, held);
            assert!(lowerer.reserve_constructor_commit(copy, 0, at).is_none());
            assert_eq!(credits(lowerer), held);
            assert!(lowerer.instructions.is_empty());
        });
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M3201");
    }
}

#[test]
fn constructor_envelope_array_and_enum_parent_failure_child_failure_and_exact_commit() {
    for (fixture, statement, costs) in [
        (Fixture::Array, 0, [2, 3, 3, 3]),
        (Fixture::Enum, 1, [1, 2, 2, 2]),
        (Fixture::EmptyEnum, 0, [0, 1, 1, 1]),
    ] {
        for mode in 0..3 {
            let errors = with_fixture(fixture, |lowerer, result| {
                for index in 0..statement {
                    assert!(run_statement(lowerer, index, result));
                }
                let current = [
                    lowerer.aggregate_operands,
                    lowerer.instructions.len(),
                    lowerer.next_value as usize,
                    lowerer.places.len(),
                ];
                let mut held =
                    std::array::from_fn(|index| LIMITS[index] - current[index] - costs[index]);
                if mode == 0 {
                    held[2] = LIMITS[2] - current[2];
                }
                if mode == 1 {
                    held[2] += 1;
                }
                set_credits(lowerer, held);
                let before = child_preparation_red::state(lowerer);
                let value = lowerer.value(root_value(lowerer, statement), result);
                assert_eq!(credits(lowerer), held);
                assert_eq!(value.is_some(), mode == 2);
                if mode != 2 {
                    assert_eq!(child_preparation_red::state(lowerer), before);
                }
                if mode == 2 {
                    assert_eq!(lowerer.next_value as usize, current[2] + costs[2]);
                    assert_eq!(lowerer.places.len(), current[3] + costs[3]);
                    assert_eq!(lowerer.instructions.len(), current[1] + costs[1]);
                    assert_eq!(lowerer.aggregate_operands, current[0] + costs[0]);
                }
            });
            assert_eq!(errors.len(), usize::from(mode != 2));
        }
    }
}
