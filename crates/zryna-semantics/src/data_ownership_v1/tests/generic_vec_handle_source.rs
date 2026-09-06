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
        assert!(
            instructions[clone_at].derived_drop_actions().any(|a| a.root() == begin.container())
        );
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
            let clones = instructions[begin_at + 1..commit_at]
                .iter()
                .filter(|i| has_count_clone(**i))
                .collect::<Vec<_>>();
            assert_eq!(clones.len(), usize::from(matches!(operation, Operation::ReplaceClone)));
            for clone in clones {
                assert!(has_count_clone(*clone));
                assert!(clone.derived_drop_actions().any(|a| a.root() == begin.container()));
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
            assert!(prepared_at < push_at);
            let owner = function
                .places()
                .find(|p| p.kind() == VerifiedPlaceKind::Temporary(value))
                .expect("verified generic Vec handle shape");
            let failure = push.derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(failure[0].root(), owner.id());
            assert!(failure.iter().any(|a| a.root() == target));
            assert!(block.terminator().derived_drop_actions().all(|a| a.root() != owner.id()));
            let clones =
                instructions[..push_at].iter().filter(|i| has_count_clone(**i)).collect::<Vec<_>>();
            assert_eq!(clones.len(), usize::from(!matches!(operation, Operation::Push)));
            assert!(clones.iter().all(|i| has_count_clone(**i)));
            if matches!(operation, Operation::PushIndexedClone) {
                let end = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .expect("verified generic Vec handle shape");
                assert!(prepared_at < end && end < push_at);
            }
        }
    }
}

#[test]
fn generic_vec_handle_source_replays_identically() {
    for element in elements() {
        for operation in [Operation::Clone, Operation::ReplaceClone, Operation::PushIndexedClone] {
            let first = verified(&element, operation);
            let second = verified(&element, operation);
            let trace = |program: &VerifiedProgram| {
                program
                    .modules()
                    .next()
                    .expect("verified generic Vec handle shape")
                    .functions()
                    .next()
                    .expect("verified generic Vec handle shape")
                    .blocks()
                    .next()
                    .expect("verified generic Vec handle shape")
                    .instructions()
                    .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                    .collect::<Vec<_>>()
            };
            assert_eq!(trace(&first), trace(&second));
        }
    }
}
