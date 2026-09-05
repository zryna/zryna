use super::generic_vec_fixture::ordinary_array_composition_fixture::checked_chain_fixture::{
    Shape, fixture,
};
use super::*;
use zryna_ir::data_ownership_v1::{
    BorrowIdentity, PlaceIdentity, VerifiedGenericCloneSource, VerifiedInstruction,
};

fn bounds(
    instructions: &[VerifiedInstruction<'_>],
    shape: Shape,
    replace: bool,
    bad: Option<(usize, i32)>,
) -> (BorrowIdentity, PlaceIdentity, usize) {
    let first =
        instructions.iter().position(|i| i.indexed_borrow().is_some()).expect("first bounds");
    let begin = instructions[first].indexed_borrow().expect("begin");
    let mut sites = vec![(first, begin.borrow(), begin.index(), begin.array_length())];
    for (at, instruction) in instructions.iter().enumerate() {
        if let Some(project) = instruction.indexed_projection() {
            assert_eq!(project.parent(), sites.last().expect("parent").1);
            assert_eq!(project.container(), begin.container());
            assert_eq!(project.access(), begin.access());
            assert_eq!(project.trap_identity(), VerifiedTrapIdentity::BoundsV1);
            sites.push((at, project.borrow(), project.index(), project.array_length()));
        }
    }
    assert_eq!(sites.len(), shape.containers().len());
    assert_eq!(
        begin.access(),
        if replace { VerifiedBorrowAccess::Exclusive } else { VerifiedBorrowAccess::Shared }
    );
    for (ordinal, (at, borrow, index, length)) in sites.iter().enumerate() {
        assert_eq!(*length, if shape.containers()[ordinal] { None } else { Some(2) });
        let ended = instructions[*at].failure_ended_borrows().collect::<Vec<_>>();
        assert_eq!(
            ended,
            ordinal
                .checked_sub(1)
                .map(|previous| sites[previous].1)
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            instructions[*at].derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
            [begin.container()]
        );
        if bad.is_none_or(|(bad, _)| bad != ordinal) {
            let (call_at, call) = instructions
                .iter()
                .enumerate()
                .find(|(_, i)| {
                    i.callee().is_some_and(|callee| callee.declaration() as usize == ordinal + 2)
                })
                .expect("one index call");
            assert_eq!(call.result(), Some(*index));
            assert!(call_at < *at);
            if ordinal != 0 {
                assert!(sites[ordinal - 1].0 < call_at);
            }
            assert_eq!(call.failure_ended_borrows().collect::<Vec<_>>(), ended);
            assert_eq!(
                call.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
                [begin.container()]
            );
        }
        assert_eq!(instructions[*at].borrow(), Some(*borrow));
    }
    let last = sites.last().expect("final site");
    (last.1, begin.container(), last.0)
}

fn operation(
    instructions: &[VerifiedInstruction<'_>],
    owned: bool,
    replace: bool,
    final_access: (BorrowIdentity, PlaceIdentity, usize),
) {
    let (borrow, container, checked) = final_access;
    let end = instructions
        .iter()
        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("end");
    assert_eq!(
        instructions.iter().filter(|i| i.kind() == VerifiedInstructionKind::EndBorrow).count(),
        1
    );
    assert_eq!(instructions[end].borrow(), Some(borrow));
    if replace {
        let (rhs_at, rhs) = instructions
            .iter()
            .enumerate()
            .rev()
            .find(|(_, i)| i.callee().is_some())
            .expect("RHS call");
        let commit = instructions
            .iter()
            .position(|i| {
                i.kind()
                    == if owned {
                        VerifiedInstructionKind::BorrowReplace
                    } else {
                        VerifiedInstructionKind::BorrowWrite
                    }
            })
            .expect("replacement");
        assert!(checked < rhs_at && rhs_at < commit && commit < end);
        assert_eq!(instructions[commit].borrow(), Some(borrow));
        assert_eq!(rhs.failure_ended_borrows().collect::<Vec<_>>(), [borrow]);
        assert_eq!(
            rhs.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
            [container]
        );
        assert_eq!(
            instructions[commit].value_operands().collect::<Vec<_>>(),
            [rhs.result().expect("prepared RHS")]
        );
    } else if owned {
        let (at, instruction) = instructions
            .iter()
            .enumerate()
            .find(|(_, i)| i.generic_clone().is_some())
            .expect("clone");
        let clone = instruction.generic_clone().expect("clone authority");
        assert!(checked < at && at < end);
        assert_eq!(clone.source(), VerifiedGenericCloneSource::Borrow(borrow));
        assert_ne!(clone.destination(), container);
        assert_eq!(instruction.failure_ended_borrows().collect::<Vec<_>>(), [borrow]);
        assert_eq!(
            instruction.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
            [container]
        );
        assert_eq!(
            instruction
                .generic_clone_prefix_failure_drop_actions()
                .map(|drop| drop.root())
                .collect::<Vec<_>>(),
            [clone.destination(), container]
        );
    } else {
        let (at, read) = instructions
            .iter()
            .enumerate()
            .find(|(_, i)| i.kind() == VerifiedInstructionKind::BorrowRead)
            .expect("Copy read");
        assert!(checked < at && at < end);
        assert_eq!(read.borrow(), Some(borrow));
    }
    let drops = instructions
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind() == VerifiedInstructionKind::DropPlace)
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), usize::from(owned && !replace));
    for (at, drop) in drops {
        assert!(end < at, "scope cleanup follows authority end");
        assert_ne!(drop.place_operands().next(), Some(container));
    }
}

fn case(shape: Shape, owned: bool, replace: bool, empty: Option<usize>, bad: Option<(usize, i32)>) {
    let (source, raw) = fixture(shape, owned, replace, empty, bad);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed checked chain");
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("caller");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let calls = instructions
            .iter()
            .filter_map(|i| i.callee())
            .map(zryna_ir::data_ownership_v1::FunctionIdentity::declaration)
            .collect::<Vec<_>>();
        let mut expected = vec![1];
        expected.extend(
            (0..shape.containers().len())
                .filter(|ordinal| bad.is_none_or(|(bad, _)| *ordinal != bad))
                .map(|ordinal| u32::try_from(ordinal + 2).expect("index declaration")),
        );
        if replace {
            expected.push(u32::try_from(shape.containers().len() + 2).expect("RHS declaration"));
        }
        assert_eq!(calls, expected, "base, indices and RHS execute once in source order");
        assert_eq!(
            instructions
                .iter()
                .find(|i| i.callee().is_some())
                .expect("base call")
                .derived_drop_actions()
                .count(),
            0
        );
        let final_access = bounds(&instructions, shape, replace, bad);
        operation(&instructions, owned, replace, final_access);
        assert_eq!(
            block.terminator().derived_drop_actions().count(),
            0,
            "complete container is returned after scope cleanup"
        );
        assert!(
            instructions
                .iter()
                .flat_map(|i| i.derived_drop_actions())
                .all(|drop| drop.moved_projections().len() == 0)
        );
    }
}

#[test]
fn checked_chain_source_mixed_vec_descendants_preserve_value_and_cleanup_order() {
    for shape in [Shape::ArrayVec, Shape::VecVec, Shape::Alternating] {
        for owned in [false, true] {
            for replace in [false, true] {
                case(shape, owned, replace, None, None);
            }
        }
    }
}

#[test]
fn checked_chain_source_empty_negative_and_upper_bounds_keep_checked_authority() {
    for shape in [Shape::ArrayVec, Shape::VecVec, Shape::Alternating] {
        for owned in [false, true] {
            for replace in [false, true] {
                for (ordinal, vector) in shape.containers().iter().enumerate() {
                    if *vector {
                        case(shape, owned, replace, Some(ordinal), None);
                    }
                    for index in [-1, 3] {
                        case(shape, owned, replace, None, Some((ordinal, index)));
                    }
                }
            }
        }
    }
}
