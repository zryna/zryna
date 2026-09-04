use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, with_snapshot};
use super::*;
use crate::data_ownership_v1::scalar_operations::ScalarOperation;
use crate::data_ownership_v1::tests::mixed_copy_operators::operator_fixture;
use crate::data_ownership_v1::tests::scalar_operator_matrix::nested_scalar_fixture;
use std::panic::{AssertUnwindSafe, catch_unwind};
use zryna_layout::TypeCategory;

#[derive(Clone, Copy)]
enum Corruption {
    Order,
    Kind,
    Arity,
    Result,
    Range,
    CommitOrder,
}

#[test]
fn mixed_scalar_private_contract_rejects_order_kind_arity_result_and_range_corruption() {
    for corruption in [
        Corruption::Order,
        Corruption::Kind,
        Corruption::Arity,
        Corruption::Result,
        Corruption::Range,
        Corruption::CommitOrder,
    ] {
        let (source, snapshot) = operator_fixture(false);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            let root = root_value(lowerer, 0);
            let mut prepared =
                PreparedValue::prepare(lowerer, root, ty).expect("valid scalar plan");
            let start = prepared
                .plan
                .steps
                .iter()
                .position(|s| {
                    matches!(s.operation, Operation::ScalarEnter { kind: ScalarOperation::Sub, .. })
                })
                .expect("sub entry");
            let Operation::ScalarEnter { end, .. } = prepared.plan.steps[start].operation else {
                unreachable!("selected entry")
            };
            let commit = end - 1;
            assert!(matches!(
                prepared.plan.steps[commit].operation,
                Operation::ScalarCommit { kind: ScalarOperation::Sub, .. }
            ));
            let boolean = prepared
                .lowerer
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == TypeCategory::Bool)
                .copied()
                .expect("Bool authority");
            let expected = match corruption {
                Corruption::Order => {
                    let Operation::ScalarEnter { operands, .. } =
                        &mut prepared.plan.steps[start].operation
                    else {
                        unreachable!()
                    };
                    operands.swap(0, 1);
                    "scalar ordered immediate operand"
                }
                Corruption::Kind => {
                    let Operation::ScalarEnter { kind, .. } =
                        &mut prepared.plan.steps[start].operation
                    else {
                        unreachable!()
                    };
                    *kind = ScalarOperation::Add;
                    "scalar exact scope contract"
                }
                Corruption::Arity => {
                    let Operation::ScalarEnter { operands, .. } =
                        &mut prepared.plan.steps[start].operation
                    else {
                        unreachable!()
                    };
                    operands.pop();
                    "scalar scope arity"
                }
                Corruption::Result => {
                    prepared.plan.steps[start].ty = boolean;
                    prepared.plan.steps[commit].ty = boolean;
                    "scalar exact result type"
                }
                Corruption::Range => {
                    let Operation::ScalarEnter { end, .. } =
                        &mut prepared.plan.steps[start].operation
                    else {
                        unreachable!()
                    };
                    *end -= 1;
                    "scalar exact scope contract"
                }
                Corruption::CommitOrder => {
                    let Operation::ScalarCommit { operands, .. } =
                        &mut prepared.plan.steps[commit].operation
                    else {
                        unreachable!()
                    };
                    operands.swap(0, 1);
                    "scalar complete ordered operands"
                }
            };
            // Private corruption is an invariant test, not a post-panic rollback contract.
            let failure = catch_unwind(AssertUnwindSafe(|| prepared.consume()))
                .expect_err("corruption rejected");
            let text = failure
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| failure.downcast_ref::<&str>().copied())
                .expect("panic text");
            assert!(text.contains(expected), "{text}");
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn mixed_scalar_nested_scopes_deliver_only_immediate_results_without_owned_places() {
    let (source, snapshot) = nested_scalar_fixture();
    for _ in 0..2 {
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, ty| {
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            let root = root_value(lowerer, 0);
            let prepared = PreparedValue::prepare(lowerer, root, ty).expect("nested scalar plan");
            assert_eq!(state(prepared.lowerer), before);
            assert_eq!(prepared.lowerer.preparation_facts, facts);
            let commits = prepared
                .plan
                .steps
                .iter()
                .filter_map(|s| match &s.operation {
                    Operation::ScalarCommit { kind, operands } => {
                        assert!(s.ty.is_copy());
                        assert!(s.owners.is_empty());
                        Some((
                            *kind,
                            s.value.expect("scalar result"),
                            operands.iter().map(|(v, _)| *v).collect::<Vec<_>>(),
                        ))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                commits,
                vec![
                    (ScalarOperation::Mul, raw::ValueId(5), vec![raw::ValueId(3), raw::ValueId(4)]),
                    (ScalarOperation::Sub, raw::ValueId(6), vec![raw::ValueId(2), raw::ValueId(5)]),
                    (ScalarOperation::Neg, raw::ValueId(8), vec![raw::ValueId(7)]),
                    (ScalarOperation::Add, raw::ValueId(9), vec![raw::ValueId(6), raw::ValueId(8)]),
                    (
                        ScalarOperation::Eq,
                        raw::ValueId(12),
                        vec![raw::ValueId(10), raw::ValueId(11)]
                    ),
                ]
            );
            assert_eq!(prepared.consume(), raw::ValueId(13));
            assert_eq!(lowerer.next_value, 14);
            assert_eq!(lowerer.instructions.len(), 14);
            assert_eq!(lowerer.places.len(), 3);
            assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(2)]);
            assert!(lowerer.preparation_facts.string_bytes.is_empty());
            assert_eq!(lowerer.preparation_facts.held_cleanup, facts.held_cleanup);
        });
        assert!(errors.is_empty());
    }
}
