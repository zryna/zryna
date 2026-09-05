use super::generic_vec_fixture::ordinary_array_composition_fixture::fresh_vec_fixture::{
    Base, fixture,
};
use super::*;
use zryna_ir::data_ownership_v1::VerifiedGenericCloneSource;

#[test]
fn fresh_vec_source_retains_real_owner_through_once_evaluated_index_bounds_and_clone() {
    for base in [Base::Call, Base::Construction] {
        for owned in [false, true] {
            for (length, index) in [(2, None), (2, Some(-1)), (2, Some(2)), (0, Some(0))] {
                let (source, raw) = fixture(owned, base, length, index, owned, false);
                let sources = sources_for(&source);
                let syntax = verify_snapshot(raw, &sources).expect("authenticated fresh Vec");
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
                        .expect("bounds");
                    let begin = instructions[begin_at].indexed_borrow().expect("authority");
                    assert_eq!(begin.array_length(), None);
                    assert_eq!(begin.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                    assert_eq!(begin.access(), VerifiedBorrowAccess::Shared);
                    assert!(
                        instructions[begin_at]
                            .derived_drop_actions()
                            .any(|drop| drop.root() == begin.container())
                    );
                    assert_eq!(
                        instructions
                            .iter()
                            .filter(|i| i.kind() == VerifiedInstructionKind::InitializePlace)
                            .count(),
                        0
                    );
                    let producer = match base {
                        Base::Call => instructions
                            .iter()
                            .position(|i| i.callee().is_some_and(|c| c.declaration() == 1))
                            .expect("makeVec"),
                        Base::Construction => instructions
                            .iter()
                            .position(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
                            .expect("Vec construction"),
                    };
                    assert!(producer < begin_at);
                    assert_eq!(
                        instructions.iter().filter(|i| i.callee().is_some()).count(),
                        usize::from(matches!(base, Base::Call)) + usize::from(index.is_none())
                    );
                    if index.is_none() {
                        let index_at = instructions
                            .iter()
                            .position(|i| i.result() == Some(begin.index()))
                            .expect("index call result");
                        assert!(producer < index_at && index_at < begin_at);
                        assert!(instructions[index_at].callee().is_some());
                        assert!(
                            instructions[index_at]
                                .derived_drop_actions()
                                .any(|drop| drop.root() == begin.container())
                        );
                    }
                    let end = instructions
                        .iter()
                        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
                        .expect("end");
                    assert_eq!(instructions[end + 1].kind(), VerifiedInstructionKind::DropPlace);
                    assert!(
                        instructions[end + 1]
                            .derived_drop_actions()
                            .any(|drop| drop.root() == begin.container())
                    );
                    assert_eq!(
                        instructions
                            .iter()
                            .filter(|i| i.kind() == VerifiedInstructionKind::DropPlace)
                            .count(),
                        1
                    );
                    if owned {
                        let clone_at = instructions
                            .iter()
                            .position(|i| i.generic_clone().is_some())
                            .expect("clone");
                        assert!(begin_at < clone_at && clone_at < end);
                        let clone = instructions[clone_at].generic_clone().expect("clone view");
                        assert_eq!(
                            clone.source(),
                            VerifiedGenericCloneSource::Borrow(begin.borrow())
                        );
                        assert_ne!(clone.destination(), begin.container());
                        assert!(
                            instructions[clone_at]
                                .derived_drop_actions()
                                .any(|drop| drop.root() == begin.container())
                        );
                        assert_eq!(
                            instructions[clone_at].failure_ended_borrows().collect::<Vec<_>>(),
                            [begin.borrow()]
                        );
                    } else {
                        assert_eq!(
                            instructions
                                .iter()
                                .filter(|i| i.kind() == VerifiedInstructionKind::BorrowRead)
                                .count(),
                            1
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn fresh_vec_source_rejects_implicit_owned_read_and_fresh_mutation_deterministically() {
    for base in [Base::Call, Base::Construction] {
        for replace in [false, true] {
            let (source, raw) = fixture(true, base, 2, None, false, replace);
            let sources = sources_for(&source);
            if replace {
                let first = verify_snapshot(raw.clone(), &sources)
                    .expect_err("fresh indexed mutation is not source-place syntax");
                let second = verify_snapshot(raw, &sources)
                    .expect_err("deterministic source-place rejection");
                assert_eq!(first, second);
                assert_eq!(first.len(), 1);
                assert_eq!(first[0].code(), "ZRYNA-Y4002");
                continue;
            }
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated prohibited fresh operation");
            let reject =
                || lower(pair_input(&syntax, &sources)).expect_err("no move or fresh mutation");
            let errors = reject();
            assert!(!errors.is_empty());
            assert_eq!(errors, reject());
        }
    }
}
