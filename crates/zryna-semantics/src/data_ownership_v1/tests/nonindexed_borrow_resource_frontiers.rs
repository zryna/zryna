use super::super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::super::*;
use super::parameters;
use crate::data_ownership_v1::tests::generic_vec_fixture::{
    active_enum_resource_fixture as enum_fixture,
    exhaustive_enum_borrow_resource_fixture as match_fixture,
    nonindexed_static_resource_fixture as static_fixture,
};
use crate::data_ownership_v1::type_model::Binding;

use zryna_ir::data_ownership_v1 as ir;

#[derive(Clone, Copy)]
enum Frontier {
    Exact,
    Extra,
    Overflow,
}

#[derive(Clone, Copy, Debug)]
enum CloneDimension {
    Values,
    Places,
    Transitions,
    CleanupPlans,
    CleanupActions,
}

impl Frontier {
    const fn rejects(self) -> bool {
        !matches!(self, Self::Exact)
    }
}

fn alias_statement(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> u32 {
    lowerer.function.body.blocks[1].statements[0]
}

fn lower_alias_at_transition_frontier(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    frontier: Frontier,
) -> bool {
    let id = alias_statement(lowerer);
    let statement = lowerer.function.body.statements[id as usize].clone();
    let exact = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 3;
    lowerer.reserved_transitions = match frontier {
        Frontier::Exact => exact,
        Frontier::Extra => exact + 1,
        Frontier::Overflow => usize::MAX,
    };
    let before = state(lowerer);
    let accepted = lowerer.lower_lexical_declaration(&statement).is_some();
    if !accepted {
        assert_eq!(state(lowerer), before, "rejected alias admission must be atomic");
    }
    lowerer.reserved_transitions = 0;
    accepted
}

fn refined_clone_expression(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> u32 {
    lowerer
        .function
        .body
        .expressions
        .iter()
        .position(|expression| {
            let zryna_syntax::v4::RawExpressionKind::Clone { value, .. } = expression.kind else {
                return false;
            };
            let Some(borrow) = lowerer.function.body.expressions.get(value as usize) else {
                return false;
            };
            let (zryna_syntax::v4::RawExpressionKind::Borrow { value: target, .. }
            | zryna_syntax::v4::RawExpressionKind::BorrowMut { value: target, .. }) = borrow.kind
            else {
                return false;
            };
            matches!(
                lowerer.function.body.expressions.get(target as usize).map(|target| &target.kind),
                Some(zryna_syntax::v4::RawExpressionKind::Reference { name })
                    if name.text == "payload"
            )
        })
        .and_then(|id| u32::try_from(id).ok())
        .expect("exhaustive match payload borrow clone")
}

fn bind_refined_payload(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, payload: Ty) {
    parameters(lowerer);
    let source = lowerer.bindings.get("source").expect("source parameter").clone();
    let place = raw::PlaceId(u32::try_from(lowerer.places.len()).expect("short fixture"));
    let span = lowerer.places[source.place.0 as usize].span;
    lowerer.places.push(raw::Place {
        id: place,
        ty: payload.ir,
        span,
        kind: raw::PlaceKind::EnumPayload { base: source.place, variant: 1 },
    });
    lowerer.bindings.insert("payload".into(), Binding { ty: payload, place, mutable: false });
}

fn seed_clone_frontier(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    dimension: CloneDimension,
    extra: usize,
) {
    match dimension {
        CloneDimension::Values => lowerer.set_reserved_constructor_values_for_test(
            ir::MAX_VALUES_PER_FUNCTION - lowerer.budget_values() - 2 + extra,
        ),
        CloneDimension::Places => lowerer.set_reserved_constructor_places_for_test(
            ir::MAX_PLACES_PER_FUNCTION - lowerer.budget_places() - 1 + extra,
        ),
        CloneDimension::Transitions => {
            lowerer.reserved_transitions =
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 4 + extra;
        }
        CloneDimension::CleanupPlans => {
            let span = lowerer.places[0].span;
            let count = ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 2 + extra;
            lowerer.cleanup_plans.extend((0..count).map(|id| raw::CleanupPlan {
                id: raw::CleanupPlanId(u32::try_from(id).expect("cleanup limit fits u32")),
                span,
                actions: Vec::new(),
            }));
        }
        CloneDimension::CleanupActions => {
            let pending = lowerer.owners.pending().len();
            let required = pending + pending + 1;
            lowerer.cleanup_actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION - required + extra;
        }
    }
}

fn reset_clone_frontier(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    dimension: CloneDimension,
) {
    match dimension {
        CloneDimension::Values => lowerer.set_reserved_constructor_values_for_test(0),
        CloneDimension::Places => lowerer.set_reserved_constructor_places_for_test(0),
        CloneDimension::Transitions => lowerer.reserved_transitions = 0,
        CloneDimension::CleanupPlans => lowerer.cleanup_plans.clear(),
        CloneDimension::CleanupActions => lowerer.cleanup_actions = 0,
    }
}

fn clone_dimension_total(
    lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>,
    dimension: CloneDimension,
) -> usize {
    match dimension {
        CloneDimension::Values => lowerer.budget_values(),
        CloneDimension::Places => lowerer.budget_places(),
        CloneDimension::Transitions => lowerer.budget_transitions(),
        CloneDimension::CleanupPlans => lowerer.cleanup_plans.len(),
        CloneDimension::CleanupActions => lowerer.cleanup_actions,
    }
}

#[test]
fn static_and_active_enum_borrow_lowering_hit_exact_transition_capacity_and_recover() {
    for frontier in [Frontier::Exact, Frontier::Extra, Frontier::Overflow, Frontier::Exact] {
        let (source, snapshot) = static_fixture();
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            parameters(lowerer);
            assert!(run_statement(lowerer, 0, result));
            assert_eq!(lower_alias_at_transition_frontier(lowerer, frontier), !frontier.rejects());
        });
        assert_eq!(errors.len(), usize::from(frontier.rejects()), "{errors:?}");
        if frontier.rejects() {
            assert_eq!(errors[0].code(), "ZRYNA-M3201");
        }

        let (source, snapshot) = enum_fixture();
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            assert_eq!(lower_alias_at_transition_frontier(lowerer, frontier), !frontier.rejects());
        });
        assert_eq!(errors.len(), usize::from(frontier.rejects()), "{errors:?}");
        if frontier.rejects() {
            assert_eq!(errors[0].code(), "ZRYNA-M3201");
        }
    }
}

#[test]
fn exhaustive_match_payload_borrow_clone_hits_exact_resource_costs_and_first_extra_recovers() {
    for dimension in [
        CloneDimension::Values,
        CloneDimension::Places,
        CloneDimension::Transitions,
        CloneDimension::CleanupPlans,
        CloneDimension::CleanupActions,
    ] {
        let (source, snapshot) = match_fixture();
        let exact = with_snapshot(&source, snapshot, |lowerer, result| {
            bind_refined_payload(lowerer, result);
            let clone = refined_clone_expression(lowerer);
            seed_clone_frontier(lowerer, dimension, 0);
            assert!(lowerer.structured_refined_borrow_clone(clone, result).is_some());
            let limit = match dimension {
                CloneDimension::Values => ir::MAX_VALUES_PER_FUNCTION,
                CloneDimension::Places => ir::MAX_PLACES_PER_FUNCTION,
                CloneDimension::Transitions => ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                CloneDimension::CleanupPlans => ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
                CloneDimension::CleanupActions => ir::MAX_DROP_ACTIONS_PER_FUNCTION,
            };
            assert_eq!(clone_dimension_total(lowerer, dimension), limit);
        });
        assert!(exact.is_empty(), "{exact:?}");

        let (source, snapshot) = match_fixture();
        let rejected = with_snapshot(&source, snapshot, |lowerer, result| {
            bind_refined_payload(lowerer, result);
            let clone = refined_clone_expression(lowerer);
            seed_clone_frontier(lowerer, dimension, 1);
            let before = state(lowerer);
            assert!(lowerer.structured_refined_borrow_clone(clone, result).is_none());
            assert_eq!(state(lowerer), before, "first-extra rejection must be atomic");
            reset_clone_frontier(lowerer, dimension);
            assert!(
                lowerer.structured_refined_borrow_clone(clone, result).is_some(),
                "same authenticated candidate recovers"
            );
        });
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].code(), "ZRYNA-M3201");
    }
}

#[test]
fn exhaustive_match_payload_borrow_clone_overflow_is_atomic_and_recovers() {
    for dimension in [
        CloneDimension::Values,
        CloneDimension::Places,
        CloneDimension::Transitions,
        CloneDimension::CleanupActions,
    ] {
        let (source, snapshot) = match_fixture();
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            bind_refined_payload(lowerer, result);
            let clone = refined_clone_expression(lowerer);
            match dimension {
                CloneDimension::Values => {
                    lowerer.set_reserved_constructor_values_for_test(usize::MAX);
                }
                CloneDimension::Places => {
                    lowerer.set_reserved_constructor_places_for_test(usize::MAX);
                }
                CloneDimension::Transitions => lowerer.reserved_transitions = usize::MAX,
                CloneDimension::CleanupActions => lowerer.cleanup_actions = usize::MAX,
                CloneDimension::CleanupPlans => {
                    unreachable!("physical arena length cannot overflow")
                }
            }
            let before = state(lowerer);
            assert!(lowerer.structured_refined_borrow_clone(clone, result).is_none());
            assert_eq!(state(lowerer), before, "overflow rejection must be atomic");
            reset_clone_frontier(lowerer, dimension);
            assert!(
                lowerer.structured_refined_borrow_clone(clone, result).is_some(),
                "same authenticated candidate recovers"
            );
        });
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code(), "ZRYNA-M3201");
    }
}
