use super::super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::super::*;
use super::parameters;
use crate::data_ownership_v1::tests::generic_vec_fixture::{
    active_enum_resource_fixture as enum_fixture,
    nonindexed_static_resource_fixture as static_fixture,
};

use zryna_ir::data_ownership_v1 as ir;

#[derive(Clone, Copy)]
enum Frontier {
    Exact,
    Extra,
    Overflow,
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
