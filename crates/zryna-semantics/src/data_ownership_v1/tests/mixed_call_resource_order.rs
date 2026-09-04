use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement, with_snapshot};
use super::cleanup_frontiers::seed_external;
use super::*;
use crate::data_ownership_v1::owned_lowering_resources::CleanupUsage;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::mixed_call_string_nesting::{
    CallStringNesting, nested_call_fixture,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::{RawExpressionKind, RawIdentifierSyntax};

#[derive(Clone, Copy)]
enum Pressure {
    None,
    Values,
    Places,
    Transitions,
    Cleanup,
}

#[test]
fn mixed_call_later_invalid_compound_argument_precedes_reserved_resource_failures() {
    let (mut source, mut snapshot) = nested_call_fixture(CallStringNesting::StringArgument);
    let right = snapshot.files[0].functions[0].body.expressions.iter_mut().find(|e|
        matches!(&e.kind, RawExpressionKind::StringLiteral { spelling } if spelling == "\"b\""))
        .expect("actual later concat operand inside identity argument");
    let bad = right.span;
    let range = usize::try_from(bad.start).expect("span")..usize::try_from(bad.end).expect("span");
    assert_eq!(&source[range.clone()], "\"b\"");
    source.replace_range(range, "bad");
    right.kind = RawExpressionKind::Reference {
        name: RawIdentifierSyntax { text: "bad".into(), span: bad },
    };
    for pressure in [
        Pressure::None,
        Pressure::Values,
        Pressure::Places,
        Pressure::Transitions,
        Pressure::Cleanup,
    ] {
        for _ in 0..2 {
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
                assert!(run_statement(lowerer, 0, ty));
                let (plans, actions) = if matches!(pressure, Pressure::Cleanup) {
                    (ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 1, ir::MAX_DROP_ACTIONS_PER_FUNCTION - 3)
                } else {
                    (lowerer.cleanup_plans.len(), lowerer.cleanup_actions)
                };
                seed_external(lowerer, plans, actions);
                let mut tickets = Vec::new();
                let assignments = if matches!(pressure, Pressure::Transitions) {
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                } else {
                    0
                };
                match pressure {
                    Pressure::Values => {
                        for _ in 0..ir::MAX_VALUES_PER_FUNCTION {
                            tickets.push(
                                lowerer
                                    .credit_ledger()
                                    .acquire_constructor(0, 0)
                                    .expect("actual value/transition reservation"),
                            );
                        }
                    }
                    Pressure::Places => tickets.push(
                        lowerer
                            .credit_ledger()
                            .acquire_constructor(0, ir::MAX_PLACES_PER_FUNCTION)
                            .expect("actual place reservation"),
                    ),
                    Pressure::Transitions => {
                        for _ in 0..assignments {
                            lowerer.credit_ledger().acquire_assignment();
                        }
                    }
                    Pressure::None | Pressure::Cleanup => {}
                }
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3002",
                    span(lowerer.input.sources(), bad),
                    "String operand 'bad' is not declared",
                    "reference one exact preceding String local",
                ));
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let root = root_value(lowerer, 1);
                assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_facts, facts);
                for ticket in tickets.into_iter().rev() {
                    ticket.release(lowerer);
                }
                for _ in 0..assignments {
                    lowerer.credit_ledger().release_assignment();
                }
                lowerer.preparation_facts.held_cleanup =
                    CleanupUsage::release(lowerer.preparation_facts.held_cleanup, 3);
                assert!(lowerer.constructor_storage_is_clear());
                assert_eq!(lowerer.reserved_transitions, 0);
                assert_eq!(lowerer.preparation_facts.held_cleanup, [0, 0]);
            });
            assert_eq!(errors, [expected.expect("source-bound semantic diagnostic")]);
        }
    }
}
