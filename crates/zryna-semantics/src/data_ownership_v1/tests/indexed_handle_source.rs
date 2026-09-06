use super::explicit_indexed_fixture::{Action, Container, call_fixture, fixture};
use super::generic_vec_fixture::Element;
use super::*;

pub(super) fn elements() -> [Element; 6] {
    [
        Element::Shared,
        Element::Weak,
        Element::HandleStruct,
        Element::HandleEnum,
        Element::HandleArray,
        Element::HandleVec,
    ]
}

fn clone_contract(instruction: FaultVerifiedInstruction<'_>, root: FaultPlaceIdentity) {
    let clone = instruction.handle_aware_clone().expect("handle-aware clone");
    assert_eq!(clone.source_authority(), VerifiedHandleAwareCloneSourceAuthority::Root(root));
    assert_ne!(clone.destination(), root);
    assert!(clone.frontier().nodes().any(|node| matches!(
        node.kind(),
        VerifiedHandleCloneRecipeKind::SharedCountClone
            | VerifiedHandleCloneRecipeKind::WeakCountClone
    )));
    let prepare = instruction.derived_drop_actions().collect::<Vec<_>>();
    let prefix = instruction.handle_aware_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(prefix.len(), prepare.len() + 1);
    assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
    assert_eq!(prefix[0].root(), clone.destination());
    assert_eq!(
        prefix[1..]
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        prepare
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>()
    );
    assert!(prepare.iter().any(|a| a.root() == root));
    assert!(prepare.iter().all(|a| a.moved_projections().len() == 0));
}

#[test]
fn indexed_handle_source_shared_and_exclusive_clones_preserve_exact_authority() {
    for container in [Container::Array(2), Container::Array(0), Container::Vec] {
        for element in elements() {
            for index in [None, Some(-1), Some(i32::MAX)] {
                for exclusive in [false, true] {
                    let (source, raw) =
                        fixture(container, &element, exclusive, Action::Clone, index);
                    let sources = sources_for(&source);
                    let syntax =
                        verify_snapshot(raw, &sources).expect("authenticated handle alias");
                    let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                        panic!("{container:?} {element:?} {exclusive}: {errors:?}\n{source}")
                    });
                    let function = program
                        .modules()
                        .next()
                        .expect("verified handle fixture")
                        .functions()
                        .next()
                        .expect("verified handle fixture");
                    let block = function.blocks().next().expect("verified handle fixture");
                    let instructions = block.instructions().collect::<Vec<_>>();
                    let begin = instructions
                        .iter()
                        .position(|i| i.indexed_borrow().is_some())
                        .expect("verified handle fixture");
                    let authority =
                        instructions[begin].indexed_borrow().expect("verified handle fixture");
                    assert_eq!(
                        authority.access(),
                        if exclusive {
                            VerifiedBorrowAccess::Exclusive
                        } else {
                            VerifiedBorrowAccess::Shared
                        }
                    );
                    assert_eq!(
                        authority.array_length(),
                        match container {
                            Container::Array(length) => Some(length),
                            Container::Vec => None,
                        }
                    );
                    assert_eq!(
                        instructions[..begin]
                            .iter()
                            .filter(|i| i.result() == Some(authority.index()))
                            .count(),
                        1
                    );
                    assert_eq!(instructions[begin].failure_ended_borrows().len(), 0);
                    let read = instructions
                        .iter()
                        .position(|i| i.handle_aware_clone().is_some())
                        .expect("verified handle fixture");
                    assert!(read > begin);
                    let clone =
                        instructions[read].handle_aware_clone().expect("verified handle fixture");
                    assert_eq!(clone.ty(), authority.referent());
                    assert_eq!(instructions[read].borrow(), Some(authority.borrow()));
                    assert_eq!(
                        instructions[read].failure_ended_borrows().collect::<Vec<_>>(),
                        [authority.borrow()]
                    );
                    clone_contract(instructions[read], authority.container());
                    let end = instructions
                        .iter()
                        .position(|i| {
                            i.kind() == VerifiedInstructionKind::EndBorrow
                                && i.borrow() == Some(authority.borrow())
                        })
                        .expect("verified handle fixture");
                    assert!(end > read);
                    assert!(
                        !block
                            .terminator()
                            .derived_drop_actions()
                            .any(|a| a.root() == authority.container()),
                        "restored complete container transfers into the return"
                    );
                    assert_eq!(
                        format!("{program:?}"),
                        format!(
                            "{:?}",
                            lower(pair_input(&syntax, &sources)).expect("verified handle fixture")
                        )
                    );
                }
            }
        }
    }
}

#[test]
fn indexed_handle_source_replacement_prepares_rhs_before_exact_referent_drop() {
    for container in [Container::Array(2), Container::Vec] {
        for element in elements() {
            for action in [Action::Replace, Action::ReplaceClone] {
                let (source, raw) = fixture(container, &element, true, action, None);
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated handle replacement");
                let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                    panic!("{container:?} {element:?} {action:?}: {errors:?}\n{source}")
                });
                let function = program
                    .modules()
                    .next()
                    .expect("verified handle fixture")
                    .functions()
                    .next()
                    .expect("verified handle fixture");
                let block = function.blocks().next().expect("verified handle fixture");
                let instructions = block.instructions().collect::<Vec<_>>();
                let begin = instructions
                    .iter()
                    .position(|i| i.indexed_borrow().is_some())
                    .expect("verified handle fixture");
                let authority =
                    instructions[begin].indexed_borrow().expect("verified handle fixture");
                let commit = instructions
                    .iter()
                    .position(|i| i.borrow_replacement().is_some())
                    .expect("verified handle fixture");
                let view =
                    instructions[commit].borrow_replacement().expect("verified handle fixture");
                assert_eq!(view.referent(), authority.referent());
                assert_eq!(view.old_value_drop().borrow(), authority.borrow());
                assert_eq!(instructions[commit].borrow(), Some(authority.borrow()));
                assert_eq!(
                    instructions[commit].derived_drop_actions().len(),
                    0,
                    "never drop the container"
                );
                assert!(instructions[commit].cleanup().is_none());
                assert_eq!(instructions[commit].failure_ended_borrows().len(), 0);
                assert!(commit > begin + 1, "RHS evaluation is inside the exclusive lifetime");
                if matches!(action, Action::ReplaceClone) {
                    let rhs = instructions[begin + 1..commit]
                        .iter()
                        .find(|i| i.cleanup().is_some())
                        .expect("verified handle fixture");
                    assert_eq!(
                        rhs.failure_ended_borrows().collect::<Vec<_>>(),
                        [authority.borrow()]
                    );
                    let cleanup = rhs.derived_drop_actions().collect::<Vec<_>>();
                    assert!(cleanup.iter().any(|a| a.root() == authority.container()));
                    assert!(cleanup.iter().all(|a| a.moved_projections().len() == 0));
                    assert_eq!(
                        cleanup.len(),
                        2,
                        "old container and distinct RHS source remain owned"
                    );
                }
                assert!(
                    instructions[commit + 1..]
                        .iter()
                        .any(|i| i.kind() == VerifiedInstructionKind::EndBorrow
                            && i.borrow() == Some(authority.borrow()))
                );
                assert!(
                    !block
                        .terminator()
                        .derived_drop_actions()
                        .any(|a| a.root() == authority.container())
                );
            }
        }
    }
}

#[test]
fn indexed_handle_source_calls_preserve_caller_owned_formal_clone_and_replacement() {
    for container in [Container::Array(2), Container::Vec] {
        for element in elements() {
            for exclusive in [false, true] {
                let (source, raw) = call_fixture(container, &element, exclusive);
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated indexed handle call");
                let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                    panic!("{element:?} {exclusive}: {errors:?}\n{source}")
                });
                let module = program.modules().next().expect("verified handle fixture");
                let caller = module.functions().next().expect("verified handle fixture");
                let body = caller.blocks().next().expect("verified handle fixture");
                let instructions = body.instructions().collect::<Vec<_>>();
                let begin = instructions
                    .iter()
                    .find_map(|i| i.indexed_borrow())
                    .expect("verified handle fixture");
                let call = instructions
                    .iter()
                    .find(|i| i.kind() == VerifiedInstructionKind::DirectCall)
                    .expect("verified handle fixture");
                assert_eq!(call.failure_ended_borrows().collect::<Vec<_>>(), [begin.borrow()]);
                assert!(call.derived_drop_actions().any(|a| a.root() == begin.container()));
                assert!(
                    instructions.iter().any(|i| i.kind() == VerifiedInstructionKind::EndBorrow
                        && i.borrow() == Some(begin.borrow()))
                );
                let recipient = module.functions().nth(1).expect("verified handle fixture");
                let instructions = recipient
                    .blocks()
                    .next()
                    .expect("verified handle fixture")
                    .instructions()
                    .collect::<Vec<_>>();
                let read = instructions
                    .iter()
                    .find(|i| i.handle_aware_clone().is_some())
                    .expect("verified handle fixture");
                let clone = read.handle_aware_clone().expect("verified handle fixture");
                let borrow = read.borrow().expect("verified handle fixture");
                assert_eq!(clone.ty(), begin.referent());
                assert_eq!(
                    clone.source_authority(),
                    VerifiedHandleAwareCloneSourceAuthority::FormalBorrow(borrow)
                );
                assert_eq!(read.failure_ended_borrows().collect::<Vec<_>>(), [borrow]);
                let replacement = instructions.iter().find_map(|i| i.borrow_replacement());
                assert_eq!(replacement.is_some(), exclusive);
                if let Some(replacement) = replacement {
                    assert_eq!(replacement.referent(), clone.ty());
                    assert_eq!(replacement.old_value_drop().borrow(), borrow);
                }
            }
        }
    }
}
