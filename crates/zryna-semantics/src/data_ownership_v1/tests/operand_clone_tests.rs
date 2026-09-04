use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::clone_decisions::CloneUsage;

fn clone_input(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> (u32, Span) {
    let clone = root_value(lowerer, 1);
    let RawExpressionKind::Clone { value, .. } =
        lowerer.expression(clone).expect("clone expression").kind
    else {
        panic!("clone")
    };
    (value, expression_span(lowerer, clone))
}

#[test]
fn operand_decisions_structural_clone_semantic_errors_precede_resource_errors() {
    for mode in 0..5 {
        let mut expected_at = None;
        let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let (mut operand, at) = clone_input(lowerer);
            let name = reference(lowerer, operand);
            let mut expected = result;
            match mode {
                0 => expected = ty(lowerer, TypeCategory::Bool),
                1 => operand = 0,
                2 => {
                    lowerer.bindings.remove(&name.text);
                }
                3 => {
                    let copy = ty(lowerer, TypeCategory::Bool);
                    lowerer.bindings.get_mut(&name.text).expect("binding").ty = copy;
                }
                4 => {
                    lowerer.partial_roots.insert(lowerer.bindings[&name.text].place);
                }
                _ => unreachable!(),
            }
            expected_at = Some(if mode == 0 { at } else { expression_span(lowerer, operand) });
            let mut usage = lowerer.clone_usage();
            usage.pending = usize::MAX; // Synthetic usage view, not an owner-vector claim.
            let before = state(lowerer);
            assert!(
                lowerer
                    .operand_decisions()
                    .aggregate_clone_decision(operand, expected, at, &usage)
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        let (code, message) = [
            (
                "ZRYNA-M3016",
                "structural clone requires one exact supported String-bearing aggregate",
            ),
            ("ZRYNA-M3013", "structural clone requires an addressable aggregate local root"),
            ("ZRYNA-M3002", "aggregate binding 'p' is not declared in this function"),
            ("ZRYNA-M3016", "structural clone source has the wrong exact aggregate type"),
            ("ZRYNA-M3014", "aggregate value 'p' is moved or only partially available"),
        ][mode];
        let help = [
            "clone an acyclic private Struct, Enum, or fixed array containing only bool, i32, String, and supported aggregate nodes",
            "clone one available aggregate local by name",
            "clone one preceding available aggregate local",
            "clone a local with the exact contextual aggregate type",
            "clone the aggregate only before moving any owned projection",
        ][mode];
        assert_error(&errors, code, message, help, expected_at.expect("span"));
    }
}

#[test]
fn operand_decisions_structural_clone_resource_frontiers_and_overflow_are_exact() {
    for dimension in 0..5 {
        for excess in [0, 1] {
            let mut at = None;
            let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
                assert!(run_statement(lowerer, 0, result));
                let (operand, site) = clone_input(lowerer);
                at = Some(site);
                let pending = lowerer.owners.pending().len();
                let mut usage = CloneUsage {
                    values: ir::MAX_VALUES_PER_FUNCTION - 1,
                    places: ir::MAX_PLACES_PER_FUNCTION - 1,
                    transitions: ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2,
                    reserved_transitions: 1,
                    cleanup_plans: ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 2,
                    cleanup_actions: ir::MAX_DROP_ACTIONS_PER_FUNCTION - (2 * pending + 1),
                    pending,
                };
                match dimension {
                    0 => usage.values += excess,
                    1 => usage.places += excess,
                    2 => usage.reserved_transitions += excess,
                    3 => usage.cleanup_plans += excess,
                    4 => usage.cleanup_actions += excess,
                    _ => unreachable!(),
                }
                let before = state(lowerer);
                let selected = lowerer
                    .operand_decisions()
                    .aggregate_clone_decision(operand, result, site, &usage);
                assert_eq!(selected.is_some(), excess == 0);
                assert_eq!(state(lowerer), before);
            });
            if excess == 0 {
                assert!(errors.is_empty());
            } else {
                assert_error(
                    &errors,
                    "ZRYNA-M3201",
                    "structural clone exceeds a checked value, place, or cleanup resource limit",
                    "reduce simultaneously live owned aggregates or clone sites",
                    at.expect("span"),
                );
            }
        }
    }
    // Synthetic counter views exercise checked raw-plus-held usage, not valid complete IR.
    for (transitions, reserved_transitions) in [(usize::MAX, 1), (1, usize::MAX)] {
        let mut at = None;
        let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let (operand, site) = clone_input(lowerer);
            at = Some(site);
            let mut usage = lowerer.clone_usage();
            usage.transitions = transitions;
            usage.reserved_transitions = reserved_transitions;
            let before = state(lowerer);
            assert!(
                lowerer
                    .operand_decisions()
                    .aggregate_clone_decision(operand, result, site, &usage)
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        assert_error(
            &errors,
            "ZRYNA-M3201",
            "structural clone exceeds a checked value, place, or cleanup resource limit",
            "reduce simultaneously live owned aggregates or clone sites",
            at.expect("span"),
        );
    }
    for (pending, message) in [
        (usize::MAX, "aggregate clone prefix cleanup overflows its checked action count"),
        (usize::MAX - 1, "aggregate clone cleanup accounting overflows"),
    ] {
        let mut at = None;
        let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let (operand, site) = clone_input(lowerer);
            at = Some(site);
            let mut usage = lowerer.clone_usage();
            usage.pending = pending;
            let before = state(lowerer);
            assert!(
                lowerer
                    .operand_decisions()
                    .aggregate_clone_decision(operand, result, site, &usage)
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        assert_error(
            &errors,
            "ZRYNA-M3201",
            message,
            "reduce simultaneously live owned aggregates",
            at.expect("span"),
        );
    }
}

#[test]
fn operand_decisions_structural_clone_retains_source_and_two_n_plus_one_cleanup_order() {
    let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let expression = root_value(lowerer, 1);
        lowerer.value(expression, result).expect("first clone");
        let pending = lowerer.owners.pending().to_vec();
        assert_eq!(pending.len(), 2);
        let plan_start = lowerer.cleanup_plans.len();
        let action_start = lowerer.cleanup_actions;
        let expected_owner = raw::PlaceId(u32::try_from(lowerer.places.len()).expect("owner"));
        let value = lowerer.value(expression, result).expect("second helper invocation");
        let reverse =
            pending.iter().rev().copied().map(raw::DropAction::DropPlace).collect::<Vec<_>>();
        assert_eq!(lowerer.cleanup_plans[plan_start].actions, reverse);
        let mut prefix = vec![raw::DropAction::DropAggregateInitializedPrefix(expected_owner)];
        prefix.extend(reverse);
        assert_eq!(lowerer.cleanup_plans[plan_start + 1].actions, prefix);
        assert_eq!(lowerer.cleanup_actions - action_start, 2 * pending.len() + 1);
        assert_eq!(lowerer.owners.owner(value), Some(expected_owner));
        assert_eq!(&lowerer.owners.pending()[..pending.len()], pending);
        assert!(lowerer.moved_projections.is_empty());
        assert!(lowerer.partial_roots.is_empty());
    });
    assert!(errors.is_empty());
}

#[test]
fn operand_decisions_existing_source_fixtures_replay_through_independent_ir() {
    for fixture in [
        Fixture::Pair,
        Fixture::Array,
        Fixture::Enum,
        Fixture::EmptyEnum,
        Fixture::WholeClone,
        Fixture::Projection,
        Fixture::PartialTransfer,
        Fixture::NestedPartialTransfer,
        Fixture::ArrayClone,
        Fixture::EnumClone,
        Fixture::StringClone,
        Fixture::ArrayStringClone,
    ] {
        let (source, snapshot) = fixtures::snapshot(fixture);
        let sources = fixtures::sources(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("authenticated complete source");
        let mut prior = None;
        for _ in 0..2 {
            let program = ownership::lower(fixtures::input(&syntax, &sources))
                .expect("independent full IR gate");
            let kinds = program
                .modules()
                .flat_map(ir::VerifiedModule::functions)
                .flat_map(ir::VerifiedFunction::blocks)
                .flat_map(ir::VerifiedBlock::instructions)
                .map(ir::VerifiedInstruction::kind)
                .collect::<Vec<_>>();
            if let Some(prior) = &prior {
                assert_eq!(&kinds, prior);
            }
            prior = Some(kinds);
        }
    }
}
