use super::generic_vec_fixture::ordinary_array_composition_fixture::lexical_chained_fixture::fixture;
use super::*;
use zryna_ir::data_ownership_v1::VerifiedGenericCloneSource;

#[test]
fn lexical_chained_source_binds_final_exact_child_and_restores_complete_container() {
    for vector in [false, true] {
        for owned in [false, true] {
            for exclusive in [false, true] {
                for indices in [[None, None], [Some(-1), None], [None, Some(2)]] {
                    let (source, raw) = fixture(vector, owned, exclusive, indices);
                    let sources = sources_for(&source);
                    let syntax =
                        verify_snapshot(raw, &sources).expect("authenticated lexical chain");
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
                        let instructions = function
                            .blocks()
                            .next()
                            .expect("block")
                            .instructions()
                            .collect::<Vec<_>>();
                        let begin_at = instructions
                            .iter()
                            .position(|i| i.indexed_borrow().is_some())
                            .expect("outer bounds");
                        let project_at = instructions
                            .iter()
                            .position(|i| i.indexed_projection().is_some())
                            .expect("inner bounds");
                        let bind_at = instructions
                            .iter()
                            .position(|i| i.indexed_binding().is_some())
                            .expect("lexical binding");
                        let begin = instructions[begin_at].indexed_borrow().expect("begin");
                        let project =
                            instructions[project_at].indexed_projection().expect("projection");
                        let bind = instructions[bind_at].indexed_binding().expect("binding");
                        assert!(begin_at < project_at && project_at < bind_at);
                        assert_eq!(begin.array_length(), if vector { None } else { Some(2) });
                        assert_eq!(project.array_length(), Some(2));
                        assert_eq!(project.parent(), begin.borrow());
                        assert_eq!(bind.parent(), project.borrow());
                        assert_eq!(bind.container(), begin.container());
                        assert_eq!(bind.referent(), project.referent());
                        assert_eq!(
                            bind.access(),
                            if exclusive {
                                VerifiedBorrowAccess::Exclusive
                            } else {
                                VerifiedBorrowAccess::Shared
                            }
                        );
                        assert_eq!(
                            instructions[project_at].failure_ended_borrows().collect::<Vec<_>>(),
                            [begin.borrow()]
                        );
                        assert!(instructions[bind_at].cleanup().is_none());
                        assert_eq!(instructions[bind_at].failure_ended_borrows().count(), 0);
                        if exclusive {
                            let rhs_at = instructions
                                .iter()
                                .position(|i| i.callee().is_some_and(|c| c.declaration() == 3))
                                .expect("RHS call");
                            assert!(bind_at < rhs_at);
                            assert_eq!(
                                instructions[rhs_at].failure_ended_borrows().collect::<Vec<_>>(),
                                [bind.borrow()]
                            );
                            if owned || vector {
                                assert!(
                                    instructions[rhs_at]
                                        .derived_drop_actions()
                                        .any(|drop| drop.root() == begin.container())
                                );
                            }
                            let commit = instructions
                                .iter()
                                .find(|i| {
                                    i.kind()
                                        == if owned {
                                            VerifiedInstructionKind::BorrowReplace
                                        } else {
                                            VerifiedInstructionKind::BorrowWrite
                                        }
                                })
                                .expect("alias replacement");
                            assert_eq!(commit.borrow(), Some(bind.borrow()));
                        } else if owned {
                            let clone_at = instructions
                                .iter()
                                .position(|i| i.generic_clone().is_some())
                                .expect("alias clone");
                            assert_eq!(
                                instructions[clone_at].generic_clone().expect("clone").source(),
                                VerifiedGenericCloneSource::Borrow(bind.borrow())
                            );
                            assert_eq!(
                                instructions[clone_at].failure_ended_borrows().collect::<Vec<_>>(),
                                [bind.borrow()]
                            );
                        } else {
                            let read = instructions
                                .iter()
                                .find(|i| i.kind() == VerifiedInstructionKind::BorrowRead)
                                .expect("alias read");
                            assert_eq!(read.borrow(), Some(bind.borrow()));
                        }
                        let ends = instructions
                            .iter()
                            .filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                            .collect::<Vec<_>>();
                        assert_eq!(ends.len(), 1);
                        assert_eq!(ends[0].borrow(), Some(bind.borrow()));
                        assert!(
                            instructions
                                .iter()
                                .flat_map(|i| i.derived_drop_actions())
                                .all(|drop| drop.moved_projections().len() == 0)
                        );
                    }
                }
            }
        }
    }
}
