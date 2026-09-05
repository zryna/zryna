use super::explicit_indexed_fixture::{Action, Container, fixture};
use super::generic_vec_fixture::Element;
use super::*;

#[test]
#[allow(clippy::too_many_lines)]
fn explicit_indexed_source_exact_referent_bounds_and_lexical_restoration() {
    for container in [Container::Array(2), Container::Array(0), Container::Vec] {
        for element in [
            Element::I32,
            Element::String,
            Element::Struct,
            Element::Enum,
            Element::Array,
            Element::Vec,
        ] {
            for index in [None, Some(-1), Some(0), Some(i32::MAX)] {
                for exclusive in [false, true] {
                    let action =
                        if matches!(element, Element::I32) { Action::Read } else { Action::Clone };
                    let (source, raw) = fixture(container, &element, exclusive, action, index);
                    let sources = sources_for(&source);
                    let syntax = verify_snapshot(raw, &sources).unwrap_or_else(|errors| {
                        panic!("authenticated syntax: {errors:?}\n{source}")
                    });
                    for _ in 0..2 {
                        let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| panic!("{container:?} {element:?} {index:?} {exclusive}: {errors:?}\n{source}"));
                        let function = program
                            .modules()
                            .next()
                            .expect("module")
                            .functions()
                            .next()
                            .expect("function");
                        let block = function.blocks().next().expect("block");
                        let instructions = block.instructions().collect::<Vec<_>>();
                        if matches!((container, index), (Container::Array(length), Some(value)) if value >= 0 && u64::try_from(value).expect("nonnegative") < length)
                        {
                            static_access(function, exclusive);
                            continue;
                        }
                        let begin = instructions
                            .iter()
                            .position(|i| i.indexed_borrow().is_some())
                            .expect("indexed begin");
                        let authority =
                            instructions[begin].indexed_borrow().expect("exact indexed authority");
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
                        assert_eq!(instructions[begin].failure_ended_borrows().len(), 0);
                        let index_producer = instructions[..begin]
                            .iter()
                            .filter(|i| i.result() == Some(authority.index()))
                            .collect::<Vec<_>>();
                        assert_eq!(index_producer.len(), 1);
                        let end = instructions
                            .iter()
                            .position(|i| {
                                i.kind() == VerifiedInstructionKind::EndBorrow
                                    && i.borrow() == Some(authority.borrow())
                            })
                            .expect("scope ends authority");
                        assert!(end > begin);
                        let access = instructions[begin + 1..end]
                            .iter()
                            .find(|i| {
                                matches!(
                                    i.kind(),
                                    VerifiedInstructionKind::BorrowRead
                                        | VerifiedInstructionKind::GenericCloneBorrow
                                )
                            })
                            .expect("alias observation");
                        assert_eq!(access.borrow(), Some(authority.borrow()));
                        if matches!(element, Element::I32) {
                            assert_eq!(access.kind(), VerifiedInstructionKind::BorrowRead);
                        } else {
                            let clone = access.generic_clone().expect("owned alias explicit clone");
                            assert_eq!(clone.ty(), authority.referent());
                            assert_eq!(
                                access.failure_ended_borrows().collect::<Vec<_>>(),
                                [authority.borrow()]
                            );
                            assert!(
                                access
                                    .derived_drop_actions()
                                    .any(|a| a.root() == authority.container())
                            );
                        }
                        assert!(
                            !block
                                .terminator()
                                .derived_drop_actions()
                                .any(|a| a.root() == authority.container()),
                            "restored container is returned"
                        );
                        assert!(
                            instructions[begin]
                                .derived_drop_actions()
                                .all(|a| a.moved_projections().len() == 0)
                        );
                    }
                }
            }
        }
    }
}

fn static_access(function: VerifiedFunction<'_>, exclusive: bool) {
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert!(instructions.iter().all(|i| i.indexed_borrow().is_none()));
    let begin = instructions
        .iter()
        .find(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("known in-range static authority");
    assert_eq!(
        begin.borrow_access(),
        Some(if exclusive {
            VerifiedBorrowAccess::Exclusive
        } else {
            VerifiedBorrowAccess::Shared
        })
    );
    let place = begin.place_operands().next().expect("exact static element");
    assert!(function.places().any(|p| p.id() == place
        && matches!(p.kind(), VerifiedPlaceKind::FixedArrayConstant { index: 0, .. })));
    assert!(begin.cleanup().is_none(), "statically proven access has no bounds failure site");
    assert!(instructions.iter().any(|i| i.kind() == VerifiedInstructionKind::EndBorrow && i.borrow() == begin.borrow()));
    let read = instructions
        .iter()
        .find(|i| {
            matches!(
                i.kind(),
                VerifiedInstructionKind::BorrowRead | VerifiedInstructionKind::GenericCloneBorrow
            )
        })
        .expect("static alias read");
    assert_eq!(read.borrow(), begin.borrow());
}

#[test]
fn explicit_indexed_source_owned_and_copy_replacement_preserve_container_until_scope_end() {
    for container in [Container::Array(2), Container::Vec] {
        for element in [
            Element::I32,
            Element::String,
            Element::Struct,
            Element::Enum,
            Element::Array,
            Element::Vec,
        ] {
            let actions: &[Action] = if matches!(element, Element::I32) {
                &[Action::Replace]
            } else {
                &[Action::Replace, Action::ReplaceClone]
            };
            for &action in actions {
                let (source, raw) = fixture(container, &element, true, action, None);
                let sources = sources_for(&source);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated lexical replacement");
                let program = lower(pair_input(&syntax, &sources)).unwrap_or_else(|errors| {
                    panic!("{container:?} {element:?} {action:?}: {errors:?}\n{source}")
                });
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let begin =
                    instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("begin");
                let authority = instructions[begin].indexed_borrow().expect("authority");
                let commit = instructions
                    .iter()
                    .position(|i| {
                        matches!(
                            i.kind(),
                            VerifiedInstructionKind::BorrowWrite
                                | VerifiedInstructionKind::BorrowReplace
                        )
                    })
                    .expect("commit");
                assert!(commit > begin);
                assert_eq!(instructions[commit].borrow(), Some(authority.borrow()));
                assert_eq!(
                    instructions[commit].derived_drop_actions().len(),
                    0,
                    "referent replacement is never container DropPlace"
                );
                if !matches!(element, Element::I32) {
                    let replacement =
                        instructions[commit].borrow_replacement().expect("owned replacement");
                    assert_eq!(replacement.referent(), authority.referent());
                    assert_eq!(replacement.old_value_drop().borrow(), authority.borrow());
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
