use super::super::child_preparation_red::state as preparation_state;
use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::operand_decisions::ProjectionOperation;
use crate::data_ownership_v1::type_model::ProjectedAggregateMoveContext;

fn field(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, name: &str) -> u32 {
    lowerer.function.body.expressions.iter().position(|expression|
        matches!(&expression.kind, RawExpressionKind::FieldAccess { field, .. } if field.text == name))
        .and_then(|id| u32::try_from(id).ok()).expect("authenticated field expression")
}

#[test]
fn operand_decisions_projected_copy_and_string_move_preserve_masks_and_sources() {
    let errors = with_fixture(Fixture::Projection, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let string_id = field(lowerer, "first");
        let copy_id = field(lowerer, "flag");
        let string = lowerer.owned_place(string_id).expect("String path");
        let copy = lowerer.owned_place(copy_id).expect("Copy path");
        let old = state(lowerer);
        let at = expression_span(lowerer, copy_id);
        let operation = lowerer
            .operand_decisions()
            .projection_decision(copy, copy.ty, None, at)
            .expect("Copy decision");
        assert!(matches!(operation, ProjectionOperation::Copy));
        assert_eq!(state(lowerer), old);
        let pending = lowerer.owners.pending().to_vec();
        let value = lowerer.projected_value(string_id, string.ty, None).expect("String move");
        let owner = lowerer.owners.owner(value).expect("temporary");
        assert_eq!(&lowerer.owners.pending()[..pending.len()], pending);
        assert_eq!(lowerer.owners.pending().last(), Some(&owner));
        assert!(lowerer.moved_projections.contains(&string.place));
        assert!(lowerer.partial_roots.contains(&string.root));
        assert!(lowerer.projection_available(copy.place, copy.root));
        lowerer.projected_value(copy_id, copy.ty, None).expect("disjoint Copy read");
    });
    assert!(errors.is_empty());
}

#[test]
fn operand_decisions_projection_capacity_precedes_type_and_retains_prefixes() {
    for remaining in [0, 1, 2] {
        let mut at = None;
        let errors = with_fixture(Fixture::NestedPartialTransfer, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let id = field(lowerer, "text");
            let inner = field(lowerer, "inner");
            at = Some(expression_span(lowerer, if remaining == 0 { inner } else { id }));
            let count = lowerer.places.len();
            lowerer.constructor_storage.places = ir::MAX_PLACES_PER_FUNCTION - count - remaining;
            let before_instructions = lowerer.instructions.clone();
            assert!(
                lowerer.projected_value(id, result, None).is_none(),
                "wrong contextual Outer type"
            );
            assert_eq!(lowerer.places.len() - count, remaining.min(2));
            assert_eq!(lowerer.instructions, before_instructions);
        });
        if remaining < 2 {
            assert_error(
                &errors,
                "ZRYNA-M3201",
                "derived owned projection places exceed the per-function M3 limit",
                "reduce distinct private aggregate field and fixed-array projections",
                at.expect("span"),
            );
        } else {
            assert_error(
                &errors,
                "ZRYNA-M3016",
                "owned projection has the wrong exact contextual type",
                "use one exact supported Struct field or fixed-array element",
                at.expect("span"),
            );
        }
    }
}

#[test]
fn operand_decisions_aggregate_projection_context_and_site_admission_remain_exact() {
    for context in [
        None,
        Some(ProjectedAggregateMoveContext::DirectLocal),
        Some(ProjectedAggregateMoveContext::FinalReturn),
        Some(ProjectedAggregateMoveContext::ProjectedReplacement),
    ] {
        for exhausted_site in [false, true] {
            let mut at = None;
            let errors = with_fixture(Fixture::NestedPartialTransfer, |lowerer, result| {
                assert!(run_statement(lowerer, 0, result));
                let id = field(lowerer, "inner");
                let projection = lowerer.owned_place(id).expect("actual aggregate projection");
                let site = expression_span(lowerer, id);
                at = Some(site);
                lowerer.aggregate_subobject_moves = usize::from(exhausted_site);
                let before = state(lowerer);
                let selected = lowerer.operand_decisions().projection_decision(
                    projection,
                    projection.ty,
                    context,
                    site,
                );
                assert_eq!(selected.is_some(), context.is_some() && !exhausted_site);
                if let Some(selected) = selected {
                    assert!(matches!(
                        selected,
                        ProjectionOperation::Move { aggregate_subobject: true }
                    ));
                }
                assert_eq!(state(lowerer), before);
            });
            if context.is_none() {
                assert_error(
                    &errors,
                    "ZRYNA-M3016",
                    "static aggregate-subobject move requires one exact direct local or final return",
                    "initialize one exact private local or return the exact result type from the Struct field or constant fixed-array element",
                    at.expect("span"),
                );
            } else if exhausted_site {
                assert_error(
                    &errors,
                    "ZRYNA-M3016",
                    "this checkpoint admits only one aggregate-subobject move per function",
                    "move one supported Struct or fixed-array subobject into one exact direct local",
                    at.expect("span"),
                );
            } else {
                assert!(errors.is_empty());
            }
        }
    }
}

#[test]
fn operand_decisions_string_clone_resolves_before_budget_and_retains_source() {
    for fixture in [Fixture::StringClone, Fixture::ArrayStringClone] {
        for remaining in [0, 1, 2] {
            let mut at = None;
            let mut path_at = None;
            let errors = with_fixture(fixture, |lowerer, result| {
                assert!(run_statement(lowerer, 0, result));
                let (id, operand) = lowerer
                    .function
                    .body
                    .expressions
                    .iter()
                    .enumerate()
                    .find_map(|(id, expression)| {
                        if let RawExpressionKind::Clone { value, .. } = expression.kind {
                            Some((u32::try_from(id).expect("clone"), value))
                        } else {
                            None
                        }
                    })
                    .expect("authenticated clone");
                let site = expression_span(lowerer, id);
                at = Some(site);
                path_at = Some(expression_span(lowerer, operand));
                let expected = ty(lowerer, TypeCategory::String);
                let count = lowerer.places.len();
                lowerer.constructor_storage.places =
                    ir::MAX_PLACES_PER_FUNCTION - count - remaining;
                let pending = lowerer.owners.pending().to_vec();
                let masks = lowerer.moved_projections.clone();
                let plans = lowerer.cleanup_plans.len();
                let value = lowerer.clone_projected_string(operand, expected, site);
                assert_eq!(value.is_some(), remaining == 2);
                assert_eq!(lowerer.places.len() - count, remaining);
                assert_eq!(lowerer.moved_projections, masks);
                assert_eq!(&lowerer.owners.pending()[..pending.len()], pending);
                if remaining == 2 {
                    let owner = lowerer.owners.owner(value.expect("clone")).expect("owner");
                    assert_eq!(lowerer.owners.pending().last(), Some(&owner));
                    assert_eq!(
                        lowerer.cleanup_plans[plans].actions,
                        pending
                            .iter()
                            .rev()
                            .copied()
                            .map(raw::DropAction::DropPlace)
                            .collect::<Vec<_>>()
                    );
                    let projection =
                        lowerer.owned_place(operand).expect("cached path at full capacity");
                    assert!(lowerer.projection_available(projection.place, projection.root));
                    assert_eq!(lowerer.places.len() - count, 2);
                } else {
                    assert_eq!(lowerer.cleanup_plans.len(), plans);
                }
            });
            match remaining {
                0 => assert_error(
                    &errors,
                    "ZRYNA-M3201",
                    "derived owned projection places exceed the per-function M3 limit",
                    "reduce distinct private aggregate field and fixed-array projections",
                    path_at.expect("span"),
                ),
                1 => assert_error(
                    &errors,
                    "ZRYNA-M3201",
                    "projected String clone exceeds a checked value, place, transition, or cleanup limit",
                    "reduce simultaneously live owned aggregates or projected clone sites",
                    at.expect("span"),
                ),
                _ => assert!(errors.is_empty()),
            }
        }
    }
}

#[test]
fn operand_decisions_string_clone_type_and_availability_precede_budget() {
    for mode in 0..3 {
        let mut at = None;
        let errors = with_fixture(Fixture::Projection, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let id = field(lowerer, "first");
            let mut projection = lowerer.owned_place(id).expect("String projection");
            let site = expression_span(lowerer, id);
            at = Some(site);
            let expected = if mode == 0 { ty(lowerer, TypeCategory::Bool) } else { projection.ty };
            if mode == 1 {
                projection.is_root = true;
            } // Deliberate descriptor rejection case.
            if mode == 2 {
                lowerer.moved_projections.insert(projection.place);
            }
            let mut usage = lowerer.clone_usage();
            usage.values = usize::MAX;
            let before = state(lowerer);
            assert!(
                lowerer
                    .operand_decisions()
                    .string_clone_decision(projection, expected, site, &usage)
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        assert_error(
            &errors,
            if mode < 2 { "ZRYNA-M3012" } else { "ZRYNA-M3014" },
            if mode < 2 {
                "projected String clone requires one exact static String leaf"
            } else {
                "projected String clone source is moved or overlaps a moved subobject"
            },
            if mode < 2 {
                "clone an initialized Struct field or constant fixed-array String element"
            } else {
                "clone only an initialized available static String projection"
            },
            at.expect("span"),
        );
    }
}

#[test]
fn operand_decisions_string_clone_all_resource_frontiers_are_read_only() {
    for dimension in 0..5 {
        for excess in [0, 1] {
            let mut at = None;
            let errors = with_fixture(Fixture::Projection, |lowerer, result| {
                assert!(run_statement(lowerer, 0, result));
                let id = field(lowerer, "first");
                let projection = lowerer.owned_place(id).expect("canonical source");
                let site = expression_span(lowerer, id);
                at = Some(site);
                let mut usage = lowerer.clone_usage();
                usage.values = ir::MAX_VALUES_PER_FUNCTION - 1;
                usage.places = ir::MAX_PLACES_PER_FUNCTION - 1;
                usage.transitions = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2;
                usage.reserved_transitions = 1;
                usage.cleanup_plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 1;
                usage.cleanup_actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION - usage.pending;
                match dimension {
                    0 => usage.values += excess,
                    1 => usage.places += excess,
                    2 => usage.reserved_transitions += excess,
                    3 => usage.cleanup_plans += excess,
                    4 => usage.cleanup_actions += excess,
                    _ => unreachable!(),
                }
                let before = state(lowerer);
                let selected = lowerer.operand_decisions().string_clone_decision(
                    projection,
                    projection.ty,
                    site,
                    &usage,
                );
                assert_eq!(selected.is_some(), excess == 0);
                assert_eq!(state(lowerer), before);
            });
            if excess == 0 {
                assert!(errors.is_empty());
            } else {
                assert_error(
                    &errors,
                    "ZRYNA-M3201",
                    "projected String clone exceeds a checked value, place, transition, or cleanup limit",
                    "reduce simultaneously live owned aggregates or projected clone sites",
                    at.expect("span"),
                );
            }
        }
    }
    // Synthetic counter views exercise checked raw-plus-held usage, not valid complete IR.
    for (transitions, reserved_transitions) in [(usize::MAX, 1), (1, usize::MAX)] {
        let mut at = None;
        let errors = with_fixture(Fixture::Projection, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let id = field(lowerer, "first");
            let projection = lowerer.owned_place(id).expect("canonical source");
            let site = expression_span(lowerer, id);
            at = Some(site);
            let mut usage = lowerer.clone_usage();
            usage.transitions = transitions;
            usage.reserved_transitions = reserved_transitions;
            let before = state(lowerer);
            assert!(
                lowerer
                    .operand_decisions()
                    .string_clone_decision(projection, projection.ty, site, &usage)
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        assert_error(
            &errors,
            "ZRYNA-M3201",
            "projected String clone exceeds a checked value, place, transition, or cleanup limit",
            "reduce simultaneously live owned aggregates or projected clone sites",
            at.expect("span"),
        );
    }
}

#[test]
fn operand_decisions_later_child_failure_preserves_complete_prior_state() {
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Projection);
    let field = snapshot.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find_map(|expression| match &mut expression.kind {
            RawExpressionKind::FieldAccess { field, .. } if field.text == "flag" => Some(field),
            _ => None,
        })
        .expect("later Copy field");
    let field_span = field.span;
    source.replace_range(field_span.start as usize..field_span.end as usize, "lost");
    field.text = "lost".to_owned();
    for _ in 0..2 {
        let mut at = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            at = Some(span(lowerer.input.sources(), field_span));
            let before = preparation_state(lowerer);
            assert!(lowerer.value(root_value(lowerer, 1), result).is_none());
            assert_eq!(preparation_state(lowerer), before);
            assert!(lowerer.constructor_storage_is_clear());
            assert_eq!(lowerer.reserved_transitions, 0);
        });
        assert_error(
            &errors,
            "ZRYNA-M3006",
            "struct 'OwnedPair' has no field 'lost'",
            "use one exact declared field name",
            at.expect("span"),
        );
    }
}
