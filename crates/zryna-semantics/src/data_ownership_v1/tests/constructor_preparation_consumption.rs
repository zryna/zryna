use super::super::constructor_resources::tests::{root_value, run_statement, with_fixture};
use super::super::preparation_plan::{Leaf, Operation, PreparationPlan};
use super::*;
use crate::data_ownership_v1::owner_state::OwnerDelta;
use crate::data_ownership_v1::tests::constructor_envelope_fixtures::Fixture;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy)]
enum Mutation {
    RootId,
    RootType,
    EarlierCleanup,
    SwappedCloneRoles,
    MissingClonePrefix,
    ExtraCleanup,
    CleanupOnInfallibleLeaf,
    EnterType,
    EnterArity,
    CommitType,
    CommitArity,
    EnterKind,
    CommitKind,
    Range,
    ReorderedOperands,
    MissingLeaf,
    DuplicateLeaf,
    MissingRegistration,
    WrongRegistration,
    ReorderedTransfers,
    ReorderedMove,
}

fn alter(plan: &mut PreparationPlan<'_>, mutation: Mutation) {
    let leaf = plan
        .steps
        .iter()
        .position(|step| matches!(step.operation, Operation::Leaf(_)))
        .expect("authenticated fixture has a leaf");
    match mutation {
        Mutation::RootId => plan.result = raw::ValueId(u32::MAX),
        Mutation::RootType => {
            let other = plan.steps[leaf].ty;
            assert_ne!(other, plan.result_type, "fixture has distinct child/root types");
            plan.result_type = other;
        }
        Mutation::EarlierCleanup => {
            let Operation::Leaf(Leaf::AggregateClone { cleanup, .. }) =
                &mut plan.steps[leaf].operation
            else {
                panic!("whole-clone fixture");
            };
            assert!(cleanup.0 > 0, "prior statement has an earlier valid cleanup");
            *cleanup = raw::CleanupPlanId(0);
        }
        Mutation::SwappedCloneRoles => {
            let Operation::Leaf(Leaf::AggregateClone { cleanup, prefix, .. }) =
                &mut plan.steps[leaf].operation
            else {
                panic!("whole-clone fixture");
            };
            assert_ne!(cleanup, prefix);
            std::mem::swap(cleanup, prefix);
        }
        Mutation::MissingClonePrefix => {
            let index = plan
                .steps
                .iter()
                .position(|step| {
                    matches!(step.operation, Operation::Cleanup { prefix: Some(_), .. })
                })
                .expect("clone prefix event");
            plan.steps.remove(index);
        }
        Mutation::ExtraCleanup => {
            let index = plan
                .steps
                .iter()
                .position(|step| matches!(step.operation, Operation::Cleanup { .. }))
                .expect("cleanup event");
            let step = &plan.steps[index];
            let Operation::Cleanup { id, actions, prefix } = step.operation else {
                unreachable!("selected cleanup");
            };
            let extra = super::super::preparation_plan::Step {
                operation: Operation::Cleanup { id, actions, prefix },
                ty: step.ty,
                at: step.at,
                value: step.value,
                owners: step.owners.clone(),
                after: step.after,
            };
            plan.steps.insert(index + 1, extra);
        }
        Mutation::CleanupOnInfallibleLeaf => {
            assert!(matches!(plan.steps[leaf].operation, Operation::Leaf(Leaf::String { .. })));
            plan.steps[leaf].operation = Operation::Leaf(Leaf::Bool(false));
        }
        Mutation::MissingRegistration
        | Mutation::WrongRegistration
        | Mutation::ReorderedTransfers
        | Mutation::ReorderedMove => alter_owners(plan, mutation, leaf),
        _ => alter_constructor(plan, mutation, leaf),
    }
}

fn alter_constructor(plan: &mut PreparationPlan<'_>, mutation: Mutation, leaf: usize) {
    if matches!(mutation, Mutation::ReorderedOperands) {
        let Operation::Commit { values, .. } = &plan.steps.last().expect("root commit").operation
        else {
            panic!("constructor commit");
        };
        let types = values
            .iter()
            .map(|value| {
                plan.steps
                    .iter()
                    .find(|step| step.value == Some(*value))
                    .expect("actual child result")
                    .ty
            })
            .collect::<Vec<_>>();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], types[1], "same-typed permutation witness");
        assert_eq!(types[0].category, zryna_layout::TypeCategory::String);
    }
    match mutation {
        Mutation::EnterType => {
            let other = plan.steps[leaf].ty;
            assert_ne!(other, plan.steps[0].ty);
            assert!(matches!(plan.steps[0].operation, Operation::Enter { .. }));
            plan.steps[0].ty = other;
        }
        Mutation::EnterArity => {
            let Operation::Enter { arity, .. } = &mut plan.steps[0].operation else {
                panic!("constructor entry");
            };
            *arity += 1;
        }
        Mutation::CommitType => {
            let other = plan.steps[leaf].ty;
            let commit = plan.steps.last_mut().expect("root commit");
            assert!(matches!(commit.operation, Operation::Commit { .. }));
            assert_ne!(other, commit.ty);
            commit.ty = other;
        }
        Mutation::CommitArity | Mutation::ReorderedOperands => {
            let Operation::Commit { values, .. } =
                &mut plan.steps.last_mut().expect("root commit").operation
            else {
                panic!("constructor commit");
            };
            assert_eq!(values.len(), 2, "two distinct child results");
            assert_ne!(values[0], values[1]);
            if matches!(mutation, Mutation::CommitArity) {
                values.pop();
            } else {
                values.swap(0, 1);
            }
        }
        Mutation::EnterKind => {
            let Operation::Enter { kind, .. } = &mut plan.steps[0].operation else {
                panic!("constructor entry");
            };
            assert_eq!(*kind, ConstructorKind::Struct);
            *kind = ConstructorKind::FixedArray;
        }
        Mutation::CommitKind => {
            let Operation::Commit { kind, .. } =
                &mut plan.steps.last_mut().expect("root commit").operation
            else {
                panic!("constructor commit");
            };
            assert_eq!(*kind, ConstructorKind::Struct);
            *kind = ConstructorKind::FixedArray;
        }
        Mutation::Range => {
            let Operation::Enter { end, .. } = &mut plan.steps[0].operation else {
                panic!("constructor entry");
            };
            *end = 1;
        }
        Mutation::MissingLeaf | Mutation::DuplicateLeaf => alter_sequence(plan, mutation, leaf),
        _ => unreachable!("leaf mutation handled separately"),
    }
}

fn alter_sequence(plan: &mut PreparationPlan<'_>, mutation: Mutation, leaf: usize) {
    match mutation {
        Mutation::MissingLeaf => {
            let last = plan
                .steps
                .iter()
                .rposition(|step| matches!(step.operation, Operation::Leaf(_)))
                .expect("last child");
            assert!(matches!(plan.steps[last].operation, Operation::Leaf(Leaf::String { .. })));
            plan.steps.remove(last);
            // Repair range lengths so the missing consumer, not a stale range, rejects.
            for step in &mut plan.steps {
                if let Operation::Enter { end, .. } = &mut step.operation
                    && *end > last
                {
                    *end -= 1;
                }
            }
        }
        Mutation::DuplicateLeaf => {
            let step = &plan.steps[leaf];
            let Operation::Leaf(Leaf::String { bytes, cleanup }) = step.operation else {
                panic!("String child");
            };
            let duplicate = super::super::preparation_plan::Step {
                operation: Operation::Leaf(Leaf::String { bytes, cleanup }),
                ty: step.ty,
                at: step.at,
                value: step.value,
                owners: step.owners.clone(),
                after: step.after,
            };
            plan.steps.insert(leaf + 1, duplicate);
        }
        _ => unreachable!("sequence mutation only"),
    }
}

fn alter_owners(plan: &mut PreparationPlan<'_>, mutation: Mutation, leaf: usize) {
    match mutation {
        Mutation::MissingRegistration | Mutation::WrongRegistration => {
            let owners = &mut plan.steps[leaf].owners;
            assert!(matches!(owners.as_slice(), [OwnerDelta::Registered { .. }]));
            if matches!(mutation, Mutation::MissingRegistration) {
                owners.clear();
            } else {
                owners[0] = OwnerDelta::Registered { owner: raw::PlaceId(u32::MAX) };
            }
        }
        Mutation::ReorderedTransfers => {
            let step = plan.steps.last_mut().expect("root constructor");
            assert!(matches!(step.operation, Operation::Commit { .. }));
            assert!(matches!(
                step.owners.as_slice(),
                [
                    OwnerDelta::Registered { .. },
                    OwnerDelta::Transferred { .. },
                    OwnerDelta::Transferred { .. }
                ]
            ));
            assert_ne!(step.owners[1], step.owners[2], "distinct ordered child transfers");
            step.owners.swap(1, 2);
        }
        Mutation::ReorderedMove => {
            let step = &mut plan.steps[leaf];
            assert!(matches!(step.operation, Operation::Leaf(Leaf::Reference(_))));
            assert!(matches!(
                step.owners.as_slice(),
                [OwnerDelta::Registered { .. }, OwnerDelta::Renamed { .. }]
            ));
            step.owners.swap(0, 1);
        }
        _ => unreachable!("owner witness mutation only"),
    }
}

fn exercise(fixture: Fixture, previous: usize, mutation: Option<(Mutation, &str)>) {
    let errors = with_fixture(fixture, |lowerer, ty| {
        for statement in 0..previous {
            assert!(run_statement(lowerer, statement, ty), "valid preceding statement");
        }
        assert!(lowerer.errors.is_empty(), "successful fixture setup");
        let input = root_value(lowerer, previous);
        let mut prepared = PreparedValue::prepare(lowerer, input, ty).expect("valid prepared plan");
        let expected_result = prepared.plan.result;
        if let Some((mutation, message)) = mutation {
            alter(&mut prepared.plan, mutation);
            // Only consumption is caught: neither setup nor mutation can satisfy rejection.
            let panic = catch_unwind(AssertUnwindSafe(|| prepared.consume()))
                .expect_err("mutated private plan must reject");
            let text = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .expect("string invariant panic");
            let first = text.lines().next().expect("panic headline");
            assert!(
                first == message || first == format!("assertion `left == right` failed: {message}"),
                "wrong rejection: {text}"
            );
        } else {
            assert_eq!(prepared.consume(), expected_result, "unmutated control");
        }
    });
    assert!(errors.is_empty(), "private misuse is not a source diagnostic");
}

fn reject(fixture: Fixture, previous: usize, mutation: Mutation, message: &str) {
    exercise(fixture, previous, None);
    exercise(fixture, previous, Some((mutation, message)));
}

#[test]
fn constructor_preparation_consumption_root_identity_and_type_are_bound() {
    for mutation in [Mutation::RootId, Mutation::RootType] {
        reject(Fixture::Pair, 0, mutation, "prepared root result and exact type");
    }
}

#[test]
fn constructor_preparation_consumption_earlier_cleanup_and_swapped_clone_roles_reject() {
    for mutation in
        [Mutation::EarlierCleanup, Mutation::SwappedCloneRoles, Mutation::MissingClonePrefix]
    {
        reject(Fixture::WholeClone, 1, mutation, "clone cleanup role linkage");
    }
}

#[test]
fn constructor_preparation_consumption_extra_and_infallible_cleanup_reject() {
    reject(Fixture::Pair, 0, Mutation::ExtraCleanup, "prepared cleanup identity");
    reject(
        Fixture::Pair,
        0,
        Mutation::CleanupOnInfallibleLeaf,
        "infallible leaf has no cleanup events",
    );
}

#[test]
fn constructor_preparation_consumption_enter_and_commit_contracts_are_bound() {
    reject(Fixture::Pair, 0, Mutation::EnterType, "constructor release type");
    reject(Fixture::Pair, 0, Mutation::EnterArity, "prepared step effects");
    for mutation in
        [Mutation::CommitType, Mutation::CommitArity, Mutation::EnterKind, Mutation::CommitKind]
    {
        reject(Fixture::Pair, 0, mutation, "constructor commit owns exact released contract");
    }
    reject(Fixture::Pair, 0, Mutation::Range, "invalid constructor range");
}

#[test]
fn constructor_preparation_consumption_same_typed_child_order_is_bound() {
    reject(
        Fixture::Array,
        0,
        Mutation::ReorderedOperands,
        "constructor operands match ordered immediate child results",
    );
}

#[test]
fn constructor_preparation_consumption_missing_and_duplicate_leaves_reject() {
    reject(
        Fixture::Array,
        0,
        Mutation::MissingLeaf,
        "constructor release cannot interrupt cleanup effects",
    );
    reject(Fixture::Pair, 0, Mutation::DuplicateLeaf, "fallible leaf cleanup linkage");
}

#[test]
fn constructor_preparation_consumption_exact_ordered_owner_events_are_bound() {
    for mutation in
        [Mutation::MissingRegistration, Mutation::WrongRegistration, Mutation::ReorderedTransfers]
    {
        reject(Fixture::Array, 0, mutation, "prepared ordered owner effects");
    }
    reject(Fixture::Pair, 1, Mutation::ReorderedMove, "prepared ordered owner effects");
}
