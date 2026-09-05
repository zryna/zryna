use super::generic_vec_fixture::{Element, Operation, fixture};
use super::*;
use zryna_ir::data_ownership_v1::{
    BorrowIdentity, VerifiedGenericCloneSource, VerifiedTrapIdentity,
};

#[test]
fn generic_vec_source_owned_observations_clone_exact_elements_and_retain_container_cleanup() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        for index in [None, Some(-1), Some(0), Some(i32::MAX)] {
            let (source, raw) = fixture(&element, Operation::Clone, index);
            let sources = sources_for(&source);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated Vec observation source");
            let mut previous = None;
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{element:?} {index:?}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let begin_at = instructions
                    .iter()
                    .position(|i| i.indexed_borrow().is_some())
                    .expect("bounds begin");
                let clone_at = instructions
                    .iter()
                    .position(|i| i.kind() == VerifiedInstructionKind::GenericCloneBorrow)
                    .expect("exact owned observation");
                assert!(begin_at < clone_at);
                let begin = instructions[begin_at].indexed_borrow().expect("bounds authority");
                let clone = instructions[clone_at].generic_clone().expect("clone authority");
                assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                assert_eq!(begin.array_length(), None);
                assert_eq!(begin.access(), VerifiedBorrowAccess::Shared);
                assert_eq!(clone.ty(), begin.referent());
                assert!(
                    matches!(clone.source(), VerifiedGenericCloneSource::Borrow(borrow) if borrow == begin.borrow())
                );
                assert_eq!(
                    instructions
                        .iter()
                        .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .count(),
                    1
                );
                assert!(
                    instructions[clone_at]
                        .derived_drop_actions()
                        .any(|drop| drop.root() == begin.container())
                );
                assert!(
                    block
                        .terminator()
                        .derived_drop_actions()
                        .any(|drop| drop.root() == begin.container())
                );
                assert!(
                    block
                        .terminator()
                        .derived_drop_actions()
                        .all(|drop| drop.root() != clone.destination())
                );
                assert_eq!(
                    instructions[clone_at]
                        .failure_ended_borrows()
                        .map(BorrowIdentity::index)
                        .collect::<Vec<_>>(),
                    [begin.borrow().index()]
                );
                if let Some(index) = index {
                    assert_eq!(
                        instructions.iter().filter(|i| i.i32_literal() == Some(index)).count(),
                        1
                    );
                } else {
                    let reads = instructions
                        .iter()
                        .filter(|instruction| instruction.result() == Some(begin.index()))
                        .collect::<Vec<_>>();
                    assert_eq!(reads.len(), 1);
                    assert_eq!(reads[0].kind(), VerifiedInstructionKind::CopyFromPlace);
                    let source = reads[0].place_operands().next().expect("index source");
                    let place = function
                        .places()
                        .find(|place| place.id() == source)
                        .expect("index storage");
                    assert_eq!(place.kind(), VerifiedPlaceKind::Parameter(1));
                }
                let trace = instructions.iter().map(|i| i.kind()).collect::<Vec<_>>();
                if let Some(previous) = previous.replace(trace.clone()) {
                    assert_eq!(previous, trace);
                }
            }
        }
    }
}

#[test]
fn generic_vec_source_owned_replacement_checks_bounds_before_rhs_and_keeps_old_container_on_failure()
 {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        for operation in [Operation::Replace, Operation::ReplaceClone] {
            let (source, raw) = fixture(&element, operation, None);
            let sources = sources_for(&source);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated Vec replacement source");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{element:?} {operation:?}: {errors:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let begin_at = instructions
                .iter()
                .position(|i| i.indexed_borrow().is_some())
                .expect("bounds begin");
            let commit_at = instructions
                .iter()
                .position(|i| i.borrow_replacement().is_some())
                .expect("owned commit");
            assert!(begin_at < commit_at);
            let begin = instructions[begin_at].indexed_borrow().expect("bounds");
            let commit = instructions[commit_at].borrow_replacement().expect("replacement");
            assert_eq!(begin.access(), VerifiedBorrowAccess::Exclusive);
            assert_eq!(commit.borrow(), begin.borrow());
            assert_eq!(commit.referent(), begin.referent());
            assert_eq!(instructions[commit_at].derived_drop_actions().count(), 0);
            assert_eq!(instructions[commit_at].failure_ended_borrows().count(), 0);
            assert!(
                instructions[begin_at]
                    .derived_drop_actions()
                    .any(|drop| drop.root() == begin.container())
            );
            if matches!(operation, Operation::ReplaceClone) {
                let preparation = instructions[begin_at + 1..commit_at]
                    .iter()
                    .filter(|i| i.cleanup().is_some())
                    .collect::<Vec<_>>();
                assert!(!preparation.is_empty(), "fallible clone happens after bounds");
                for instruction in preparation {
                    assert!(
                        instruction
                            .derived_drop_actions()
                            .any(|drop| drop.root() == begin.container())
                    );
                    assert_eq!(
                        instruction
                            .failure_ended_borrows()
                            .map(BorrowIdentity::index)
                            .collect::<Vec<_>>(),
                        [begin.borrow().index()]
                    );
                }
            }
            assert!(
                block
                    .terminator()
                    .derived_drop_actions()
                    .all(|drop| drop.root() != begin.container())
            );
            assert_eq!(
                instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn generic_vec_source_copy_reads_and_replacements_do_not_require_owned_clone() {
    for element in [Element::I32, Element::Bool] {
        for operation in [Operation::Read, Operation::Replace] {
            let (source, raw) = fixture(&element, operation, None);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated Copy Vec source");
            let program = lower(pair_input(&syntax, &sources))
                .unwrap_or_else(|errors| panic!("{element:?} {operation:?}: {errors:?}"));
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let instructions = function
                .blocks()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
                .collect::<Vec<_>>();
            assert!(
                instructions
                    .iter()
                    .all(|i| i.kind() != VerifiedInstructionKind::GenericCloneBorrow
                        && i.borrow_replacement().is_none())
            );
            let expected = if matches!(operation, Operation::Read) {
                VerifiedInstructionKind::VecIndexCopy
            } else {
                VerifiedInstructionKind::BorrowWrite
            };
            assert_eq!(instructions.iter().filter(|i| i.kind() == expected).count(), 1);
        }
    }
}
