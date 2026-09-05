use super::generic_vec_fixture::ordinary_array_composition_fixture::{Shape, fixture};
use super::*;
use zryna_ir::data_ownership_v1::{VerifiedGenericCloneSource, VerifiedInstruction};

fn replacement_commit(instructions: &[VerifiedInstruction<'_>], owned: bool, rhs: usize) {
    let kind = if owned {
        VerifiedInstructionKind::BorrowReplace
    } else {
        VerifiedInstructionKind::BorrowWrite
    };
    let commit = instructions.iter().position(|i| i.kind() == kind).expect("commit");
    assert!(rhs < commit);
    assert_eq!(instructions[commit + 1].kind(), VerifiedInstructionKind::EndBorrow);
}

#[test]
fn ordinary_array_composition_fresh_call_is_retained_through_index_bounds_and_clone() {
    for owned in [false, true] {
        for (length, index) in [(2, None), (2, Some(-1)), (2, Some(2)), (0, Some(0))] {
            let (source, raw) = fixture(owned, Shape::Fresh, false, [length, 0], [index, None]);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated fresh array call");
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("caller");
                let instructions =
                    function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                let calls = instructions
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.callee().is_some())
                    .collect::<Vec<_>>();
                assert_eq!(calls.len(), if index.is_none() { 2 } else { 1 });
                assert_eq!(calls[0].1.callee().expect("makeArray").declaration(), 1);
                let begin_at = instructions
                    .iter()
                    .position(|i| i.indexed_borrow().is_some())
                    .expect("fresh bounds");
                let begin = instructions[begin_at].indexed_borrow().expect("authority");
                assert!(calls[0].0 < begin_at);
                assert_eq!(begin.array_length(), Some(u64::from(length)));
                assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                if index.is_none() {
                    assert_eq!(calls[1].1.callee().expect("firstIndex").declaration(), 2);
                    assert!(calls[0].0 < calls[1].0 && calls[1].0 < begin_at);
                    assert_eq!(calls[1].1.result(), Some(begin.index()));
                    if owned && length != 0 {
                        assert!(
                            calls[1]
                                .1
                                .derived_drop_actions()
                                .any(|drop| drop.root() == begin.container())
                        );
                    }
                }
                if owned {
                    let clone_at = instructions
                        .iter()
                        .position(|i| i.generic_clone().is_some())
                        .expect("owned clone");
                    let clone = instructions[clone_at].generic_clone().expect("clone view");
                    assert!(begin_at < clone_at);
                    assert_eq!(clone.source(), VerifiedGenericCloneSource::Borrow(begin.borrow()));
                    assert_eq!(
                        instructions[clone_at].failure_ended_borrows().collect::<Vec<_>>(),
                        [begin.borrow()]
                    );
                    if length != 0 {
                        assert!(
                            instructions[clone_at]
                                .derived_drop_actions()
                                .any(|drop| drop.root() == begin.container())
                        );
                    }
                } else {
                    assert_eq!(
                        instructions
                            .iter()
                            .filter(|i| i.kind() == VerifiedInstructionKind::BorrowRead)
                            .count(),
                        1
                    );
                }
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
}

#[test]
#[allow(clippy::too_many_lines)]
fn ordinary_array_composition_chained_access_preserves_region_and_atomic_authority_order() {
    for owned in [false, true] {
        for replace in [false, true] {
            for (lengths, indices) in [
                ([2, 2], [None, None]),
                ([2, 2], [Some(-1), None]),
                ([2, 2], [None, Some(2)]),
                ([0, 2], [None, None]),
                ([2, 0], [None, None]),
            ] {
                let (source, raw) = fixture(owned, Shape::Chained, replace, lengths, indices);
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated chained array");
                for _ in 0..2 {
                    let program = lower(pair_input(&syntax, &sources))
                        .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
                    let function = program
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("caller");
                    let instructions =
                        function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
                    let begin_at = instructions
                        .iter()
                        .position(|i| i.indexed_borrow().is_some())
                        .expect("outer bounds");
                    let project_at = instructions
                        .iter()
                        .position(|i| i.indexed_projection().is_some())
                        .expect("inner bounds");
                    let begin = instructions[begin_at].indexed_borrow().expect("outer authority");
                    let child =
                        instructions[project_at].indexed_projection().expect("child authority");
                    assert!(begin_at < project_at);
                    assert_eq!(begin.array_length(), Some(u64::from(lengths[0])));
                    assert_eq!(child.array_length(), Some(u64::from(lengths[1])));
                    assert_eq!(child.parent(), begin.borrow());
                    assert_eq!(child.container(), begin.container());
                    assert_eq!(child.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                    assert_eq!(
                        instructions[project_at].failure_ended_borrows().collect::<Vec<_>>(),
                        [begin.borrow()]
                    );
                    let calls = instructions
                        .iter()
                        .enumerate()
                        .filter(|(_, i)| i.callee().is_some())
                        .collect::<Vec<_>>();
                    assert_eq!(
                        calls.len(),
                        indices.iter().filter(|i| i.is_none()).count() + usize::from(replace)
                    );
                    for (at, instruction) in &calls {
                        match instruction.callee().expect("callee").declaration() {
                            1 => {
                                assert!(*at < begin_at);
                                assert_eq!(instruction.result(), Some(begin.index()));
                            }
                            2 => {
                                assert!(begin_at < *at && *at < project_at);
                                assert_eq!(instruction.result(), Some(child.index()));
                            }
                            3 => {
                                assert!(project_at < *at);
                                assert_eq!(
                                    instruction.failure_ended_borrows().collect::<Vec<_>>(),
                                    [child.borrow()]
                                );
                                if owned && lengths.iter().all(|n| *n != 0) {
                                    assert!(
                                        instruction
                                            .derived_drop_actions()
                                            .any(|drop| drop.root() == begin.container())
                                    );
                                    assert!(
                                        instruction
                                            .derived_drop_actions()
                                            .all(|drop| drop.moved_projections().len() == 0)
                                    );
                                }
                            }
                            other => panic!("unexpected callee {other}"),
                        }
                    }
                    if replace {
                        replacement_commit(&instructions, owned, calls.last().expect("RHS").0);
                    } else if owned {
                        let clone =
                            instructions.iter().find_map(|i| i.generic_clone()).expect("clone");
                        assert_eq!(
                            clone.source(),
                            VerifiedGenericCloneSource::Borrow(child.borrow())
                        );
                    }
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
    }
}
