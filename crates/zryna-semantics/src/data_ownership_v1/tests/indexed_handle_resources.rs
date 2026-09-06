use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::lexical_indexed_resources::{nested_statement, parameters};
use crate::data_ownership_v1::tests::explicit_indexed_fixture::{Action, Container, fixture};
use crate::data_ownership_v1::tests::generic_vec_fixture::Element;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawStatementKind;

#[test]
fn indexed_handle_resources_clone_exact_extra_overflow_preserve_state_and_recover() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [Element::Shared, Element::Weak, Element::HandleStruct, Element::HandleVec] {
            for excess in [0, 1, usize::MAX - 2, usize::MAX] {
                let (source, snapshot) = fixture(container, &element, false, Action::Clone, None);
                let mut expected = None;
                let errors = with_snapshot(&source, snapshot, |lowerer, result| {
                    crate::data_ownership_v1::lower(lowerer.input)
                        .expect("authenticated success control");
                    parameters(lowerer);
                    assert!(run_statement(lowerer, 0, result));
                    assert!(run_statement(lowerer, 1, result));
                    let begin = lowerer.function.body.statements
                        [nested_statement(lowerer, 0) as usize]
                        .clone();
                    lowerer.lower_lexical_declaration(&begin).expect("lexical begin");
                    let pending = lowerer.owners.pending().len();
                    assert_eq!(pending, 2, "container and separate replacement source");
                    let demand =
                        pending.checked_mul(2).and_then(|n| n.checked_add(1)).expect("clone plans");
                    lowerer.cleanup_actions = if excess > 1 {
                        excess
                    } else {
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION - demand + excess
                    };
                    let before = state(lowerer);
                    let checkpoint = lowerer.preparation_checkpoint();
                    let facts = lowerer.preparation_facts.clone();
                    let statement = nested_statement(lowerer, 1);
                    if excess != 0 {
                        let RawStatementKind::LocalDeclaration { initializer, .. } =
                            lowerer.function.body.statements[statement as usize].kind
                        else {
                            panic!("clone local");
                        };
                        let at = crate::data_ownership_v1::span(
                            lowerer.input.sources(),
                            lowerer.function.body.expressions[initializer as usize].span,
                        );
                        let (message, guidance) = if excess > 1 {
                            (
                                "owned cleanup resource accounting overflowed",
                                "reduce simultaneously live owners or cleanup sites",
                            )
                        } else {
                            (
                                "structural clone exceeds a checked value, place, or cleanup resource limit",
                                "reduce simultaneously live owned aggregates or clone sites",
                            )
                        };
                        expected = Some(Diagnostic::error_at("ZRYNA-M3201", at, message, guidance));
                    }
                    assert_eq!(run_statement(lowerer, statement as usize, result), excess == 0);
                    if excess == 0 {
                        assert_eq!(lowerer.cleanup_actions, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
                    } else {
                        assert_eq!(state(lowerer), before);
                        assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                        assert_eq!(lowerer.preparation_facts, facts);
                        lowerer.cleanup_actions = 0;
                        assert!(
                            run_statement(lowerer, statement as usize, result),
                            "pristine retry"
                        );
                    }
                    assert!(lowerer.constructor_storage_is_clear());
                    lowerer.reserved_transitions = 0;
                });
                assert_eq!(
                    errors,
                    expected.into_iter().collect::<Vec<_>>(),
                    "{container:?} {element:?} excess={excess}"
                );
            }
        }
    }
}

#[test]
fn indexed_handle_resources_replacement_commit_credit_is_atomic_and_recovers() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [Element::Shared, Element::Weak, Element::HandleEnum] {
            for extra in [false, true] {
                let (source, snapshot) = fixture(container, &element, true, Action::Replace, None);
                let errors = with_snapshot(&source, snapshot, |lowerer, result| {
                    crate::data_ownership_v1::lower(lowerer.input)
                        .expect("authenticated replacement control");
                    parameters(lowerer);
                    assert!(run_statement(lowerer, 0, result));
                    assert!(run_statement(lowerer, 1, result));
                    let begin = lowerer.function.body.statements
                        [nested_statement(lowerer, 0) as usize]
                        .clone();
                    lowerer.lower_lexical_declaration(&begin).expect("exclusive begin");
                    let end_credit = lowerer.reserved_transitions;
                    assert_eq!(end_credit, 1);
                    lowerer.reserved_transitions =
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 2
                            + usize::from(extra);
                    let before = state(lowerer);
                    let checkpoint = lowerer.preparation_checkpoint();
                    let facts = lowerer.preparation_facts.clone();
                    let statement = nested_statement(lowerer, 1);
                    let count = lowerer.instructions.len();
                    assert_eq!(run_statement(lowerer, statement as usize, result), !extra);
                    if extra {
                        assert_eq!(state(lowerer), before);
                        assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                        assert_eq!(lowerer.preparation_facts, facts);
                        lowerer.reserved_transitions = end_credit;
                        assert!(
                            run_statement(lowerer, statement as usize, result),
                            "retry with end credit retained"
                        );
                    }
                    assert_eq!(
                        lowerer.instructions.len() - count,
                        2,
                        "one RHS move then one commit"
                    );
                    assert_eq!(lowerer.preparation_facts.active_borrows, facts.active_borrows);
                    lowerer.reserved_transitions = 0;
                });
                assert_eq!(errors.len(), usize::from(extra));
                if extra {
                    assert_eq!(errors[0].code(), "ZRYNA-M3201");
                }
            }
        }
    }
}
