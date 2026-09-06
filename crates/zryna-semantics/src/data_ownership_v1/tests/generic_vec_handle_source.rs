use super::generic_vec_fixture::{Element, Operation, fixture};
use super::*;
use crate::data_ownership_v1::VerifiedProgram;
use zryna_ir::data_ownership_v1::VerifiedHandleAwareCloneSource;

fn elements() -> [Element; 6] {
    [
        Element::Shared,
        Element::Weak,
        Element::HandleStruct,
        Element::HandleEnum,
        Element::HandleArray,
        Element::HandleVec,
    ]
}

fn verified(element: &Element, operation: Operation) -> VerifiedProgram {
    let (source, raw) = fixture(element, operation, None);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated handle Vec source");
    lower(pair_input(&syntax, &sources))
        .unwrap_or_else(|errors| panic!("{element:?} {operation:?}: {errors:?}"))
}

fn has_count_clone(instruction: FaultVerifiedInstruction<'_>) -> bool {
    matches!(
        instruction.kind(),
        VerifiedInstructionKind::SharedClone | VerifiedInstructionKind::WeakClone
    ) || instruction.handle_aware_clone().is_some_and(|clone| {
        clone.frontier().nodes().any(|node| {
            matches!(
                node.kind(),
                VerifiedHandleCloneRecipeKind::SharedCountClone
                    | VerifiedHandleCloneRecipeKind::WeakCountClone
            )
        })
    })
}

#[test]
fn generic_vec_handle_observation_clones_counts_and_retains_the_complete_container() {
    for element in elements() {
        let program = verified(&element, Operation::Clone);
        let function = program
            .modules()
            .next()
            .expect("verified generic Vec handle shape")
            .functions()
            .next()
            .expect("verified generic Vec handle shape");
        let block = function.blocks().next().expect("verified generic Vec handle shape");
        let instructions = block.instructions().collect::<Vec<_>>();
        let begin_at = instructions
            .iter()
            .position(|i| i.indexed_borrow().is_some())
            .expect("verified generic Vec handle shape");
        let clone_at = instructions
            .iter()
            .position(|i| i.handle_aware_clone().is_some())
            .expect("verified generic Vec handle shape");
        let begin =
            instructions[begin_at].indexed_borrow().expect("verified generic Vec handle shape");
        let clone =
            instructions[clone_at].handle_aware_clone().expect("verified generic Vec handle shape");
        assert!(begin_at < clone_at);
        assert_eq!(begin.access(), VerifiedBorrowAccess::Shared);
        assert_eq!(clone.ty(), begin.referent());
        assert!(
            matches!(clone.source(), VerifiedHandleAwareCloneSource::Borrow(id) if id == begin.borrow())
        );
        assert!(has_count_clone(instructions[clone_at]));
        let bounds_cleanup = instructions[begin_at].derived_drop_actions().collect::<Vec<_>>();
        let clone_cleanup = instructions[clone_at].derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(clone_cleanup, bounds_cleanup);
        assert!(clone_cleanup.iter().any(|a| a.root() == begin.container()));
        let prefix = instructions[clone_at]
            .handle_aware_clone_prefix_failure_drop_actions()
            .collect::<Vec<_>>();
        assert_eq!(prefix[0].root(), clone.destination());
        assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
        assert_eq!(&prefix[1..], clone_cleanup.as_slice());
        assert_eq!(
            instructions[clone_at].failure_ended_borrows().collect::<Vec<_>>(),
            [begin.borrow()]
        );
        assert_eq!(
            instructions.iter().filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow).count(),
            1
        );
        assert!(block.terminator().derived_drop_actions().any(|a| a.root() == begin.container()));
        assert!(block.terminator().derived_drop_actions().all(|a| a.root() != clone.destination()));
        if matches!(element, Element::HandleEnum) {
            let nodes = clone.frontier().nodes().collect::<Vec<_>>();
            let root = nodes.iter().find(|node| node.ty() == clone.ty()).expect("enum recipe root");
            let VerifiedHandleCloneRecipeKind::Enum(variants) = root.kind() else {
                panic!("enum recipe")
            };
            assert_eq!(variants.len(), 1);
            assert_eq!(variants[0].0, 0);
            assert!(variants[0].1.is_some());
            assert_eq!(
                nodes
                    .iter()
                    .filter(|node| {
                        matches!(
                            node.kind(),
                            VerifiedHandleCloneRecipeKind::SharedCountClone
                                | VerifiedHandleCloneRecipeKind::WeakCountClone
                        )
                    })
                    .count(),
                2
            );
        }
    }
}

#[test]
fn generic_vec_handle_replacement_checks_bounds_then_commits_one_exact_owner() {
    for element in elements() {
        for operation in [Operation::Replace, Operation::ReplaceClone] {
            let program = verified(&element, operation);
            let function = program
                .modules()
                .next()
                .expect("verified generic Vec handle shape")
                .functions()
                .next()
                .expect("verified generic Vec handle shape");
            let block = function.blocks().next().expect("verified generic Vec handle shape");
            let instructions = block.instructions().collect::<Vec<_>>();
            let begin_at = instructions
                .iter()
                .position(|i| i.indexed_borrow().is_some())
                .expect("verified generic Vec handle shape");
            let commit_at = instructions
                .iter()
                .position(|i| i.borrow_replacement().is_some())
                .expect("verified generic Vec handle shape");
            let begin =
                instructions[begin_at].indexed_borrow().expect("verified generic Vec handle shape");
            let commit = instructions[commit_at]
                .borrow_replacement()
                .expect("verified generic Vec handle shape");
            assert!(begin_at < commit_at);
            assert_eq!(begin.access(), VerifiedBorrowAccess::Exclusive);
            assert_eq!(commit.borrow(), begin.borrow());
            assert_eq!(commit.referent(), begin.referent());
            assert_eq!(instructions[commit_at].derived_drop_actions().count(), 0);
            let producers = instructions
                .iter()
                .enumerate()
                .filter(|(_, instruction)| instruction.result() == Some(commit.value()))
                .collect::<Vec<_>>();
            assert_eq!(producers.len(), 1);
            let (prepared_at, prepared) = producers[0];
            assert!(begin_at < prepared_at && prepared_at < commit_at);
            if matches!(operation, Operation::Replace) {
                assert_eq!(prepared.kind(), VerifiedInstructionKind::MoveFromPlace);
            } else {
                assert!(has_count_clone(*prepared));
            }
            let clones = instructions[begin_at + 1..commit_at]
                .iter()
                .filter(|i| has_count_clone(**i))
                .collect::<Vec<_>>();
            assert_eq!(clones.len(), usize::from(matches!(operation, Operation::ReplaceClone)));
            for clone in clones {
                assert!(has_count_clone(*clone));
                assert_eq!(
                    clone.derived_drop_actions().collect::<Vec<_>>(),
                    instructions[begin_at].derived_drop_actions().collect::<Vec<_>>()
                );
                assert_eq!(clone.failure_ended_borrows().collect::<Vec<_>>(), [begin.borrow()]);
            }
            assert!(
                block.terminator().derived_drop_actions().all(|a| a.root() != begin.container())
            );
        }
    }
}

#[test]
fn generic_vec_handle_push_prepares_once_and_transfers_only_after_success() {
    for element in elements() {
        for operation in [Operation::Push, Operation::PushClone, Operation::PushIndexedClone] {
            let program = verified(&element, operation);
            let function = program
                .modules()
                .next()
                .expect("verified generic Vec handle shape")
                .functions()
                .next()
                .expect("verified generic Vec handle shape");
            let block = function.blocks().next().expect("verified generic Vec handle shape");
            let instructions = block.instructions().collect::<Vec<_>>();
            let push_at = instructions
                .iter()
                .position(|i| i.kind() == VerifiedInstructionKind::VecPush)
                .expect("verified generic Vec handle shape");
            let push = instructions[push_at];
            let target = push.place_operands().next().expect("verified generic Vec handle shape");
            let value = push.value_operands().next().expect("verified generic Vec handle shape");
            let prepared_at = instructions
                .iter()
                .position(|i| i.result() == Some(value))
                .expect("verified generic Vec handle shape");
            assert_eq!(instructions.iter().filter(|i| i.result() == Some(value)).count(), 1);
            assert!(prepared_at < push_at);
            let owner = function
                .places()
                .find(|p| p.kind() == VerifiedPlaceKind::Temporary(value))
                .expect("verified generic Vec handle shape");
            let failure = push.derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(failure[0].root(), owner.id());
            assert_eq!(failure.iter().filter(|a| a.root() == owner.id()).count(), 1);
            assert_eq!(failure.iter().filter(|a| a.root() == target).count(), 1);
            assert!(failure.iter().all(|a| a.moved_projections().count() == 0));
            assert!(block.terminator().derived_drop_actions().all(|a| a.root() != owner.id()));
            let clones =
                instructions[..push_at].iter().filter(|i| has_count_clone(**i)).collect::<Vec<_>>();
            assert_eq!(clones.len(), usize::from(!matches!(operation, Operation::Push)));
            assert!(clones.iter().all(|i| has_count_clone(**i)));
            if matches!(operation, Operation::Push) {
                assert_eq!(failure.len(), 2);
                assert_eq!(failure[1].root(), target);
            } else {
                let retained = instructions[prepared_at].derived_drop_actions().collect::<Vec<_>>();
                assert_eq!(&failure[1..], retained.as_slice());
            }
            if matches!(operation, Operation::PushIndexedClone) {
                let begin = instructions
                    .iter()
                    .position(|i| i.indexed_borrow().is_some())
                    .expect("indexed bounds begin");
                let end = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .expect("verified generic Vec handle shape");
                assert!(begin < prepared_at && prepared_at < end && end < push_at);
            }
        }
    }
}

#[test]
fn generic_vec_handle_source_replays_identically() {
    for element in elements() {
        for operation in [Operation::Clone, Operation::ReplaceClone, Operation::PushIndexedClone] {
            let (source, raw) = fixture(&element, operation, None);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated replay source");
            let first = lower(pair_input(&syntax, &sources)).expect("first verified replay");
            let second = lower(pair_input(&syntax, &sources)).expect("second verified replay");
            assert_eq!(format!("{first:#?}"), format!("{second:#?}"));
        }
    }
}
