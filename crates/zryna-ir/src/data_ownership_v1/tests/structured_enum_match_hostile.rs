mod fixture;

use self::fixture::{Authority, seed};
use super::*;

fn reject(authority: &Authority, valid: &raw::Program, hostile: raw::Program, code: &str) {
    let first = authority.check(hostile.clone()).expect_err("hostile structured enum match");
    assert!(first.iter().any(|error| error.code() == code), "expected {code}: {first:?}");
    assert_eq!(authority.check(hostile).expect_err("deterministic replay"), first);
    authority.check(valid.clone()).expect("recovery after hostile structured enum match");
}

#[test]
fn nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup() {
    let authority = Authority::new();
    let raw = seed(&authority);
    let verified = authority.check(raw).expect("nested multi-arm enum match");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 6);
    for (block, place, expected) in [(0, 0, [(0, 1), (1, 2)]), (2, 1, [(0, 3), (1, 4)])] {
        let terminator = blocks[block].terminator();
        assert_eq!(terminator.kind(), VerifiedTerminatorKind::EnumMatch);
        assert_eq!(terminator.place_operands().next().expect("enum discriminant").index(), place);
        assert_eq!(
            terminator
                .enum_arms()
                .map(|arm| (arm.variant(), arm.edge().target().index()))
                .collect::<Vec<_>>(),
            expected
        );
    }
    for (block, root, variant, moved) in [(1, 0, 0, 4), (2, 0, 1, 5), (3, 1, 0, 10), (4, 1, 1, 11)]
    {
        let instructions = blocks[block].instructions().collect::<Vec<_>>();
        let root_drop = instructions
            .iter()
            .flat_map(|instruction| instruction.derived_drop_actions())
            .find(|action| action.root().index() == root)
            .expect("partial enum root drop");
        assert_eq!(root_drop.active_variant(), Some(variant));
        assert!(root_drop.moved_projections().any(|place| place.index() == moved));
    }
    let cleanup = blocks[5].terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(), [3, 2]);
    assert!(cleanup.iter().all(|action| action.active_variant().is_none()));
}

#[test]
fn nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload() {
    let authority = Authority::new();
    let valid = seed(&authority);

    let mut ordinal = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut ordinal.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms[1].variant = 9;
    reject(&authority, &valid, ordinal, "ZRYNA-I3014");

    let mut shortened = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut shortened.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms.pop();
    let raw::Terminator::Jump(edge) =
        &mut shortened.modules[0].functions[0].blocks[1].terminators[0].kind
    else {
        unreachable!()
    };
    edge.target = raw::BlockId(2);
    reject(&authority, &valid, shortened, "ZRYNA-I3014");

    let mut empty = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut empty.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms.clear();
    reject(&authority, &valid, empty, "ZRYNA-I3007");

    let mut duplicate = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut duplicate.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms[1].variant = arms[0].variant;
    reject(&authority, &valid, duplicate, "ZRYNA-I3014");

    let mut absent = valid.clone();
    let raw::Terminator::EnumMatch { place, .. } =
        &mut absent.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    *place = raw::PlaceId(1);
    reject(&authority, &valid, absent, "ZRYNA-I3013");

    let mut inactive = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut inactive.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms[0].edge.target = raw::BlockId(2);
    arms[1].edge.target = raw::BlockId(1);
    reject(&authority, &valid, inactive, "ZRYNA-I3013");

    let mut foreign = valid.clone();
    foreign.modules[0].functions[0].blocks[1].instructions[0].kind =
        raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(10) };
    reject(&authority, &valid, foreign, "ZRYNA-I3013");
}

#[test]
fn nested_match_rejects_moved_partial_repeated_and_cross_arm_ownership() {
    let authority = Authority::new();
    let valid = seed(&authority);

    let mut moved = valid.clone();
    moved.modules[0].functions[0].blocks[1].instructions.swap(0, 2);
    reject(&authority, &valid, moved, "ZRYNA-I3010");

    let mut partial = valid.clone();
    partial.modules[0].functions[0].blocks[1].instructions.remove(2);
    reject(&authority, &valid, partial, "ZRYNA-I3010");

    let mut repeated = valid.clone();
    let span = repeated.modules[0].functions[0].span;
    repeated.modules[0].functions[0].blocks[1].instructions.insert(
        2,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(8) },
        },
    );
    reject(&authority, &valid, repeated, "ZRYNA-I3010");

    let mut leakage = valid.clone();
    leakage.modules[0].functions[0].blocks[4].instructions.insert(
        0,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(14) },
        },
    );
    reject(&authority, &valid, leakage, "ZRYNA-I3010");
}

#[test]
fn nested_match_rejects_incompatible_join_and_active_or_rest_cleanup_forgery() {
    let authority = Authority::new();
    let valid = seed(&authority);

    let mut incompatible = valid.clone();
    incompatible.modules[0].functions[0].blocks[1].instructions.remove(3);
    reject(&authority, &valid, incompatible, "ZRYNA-I3010");

    for mutation in 0..3 {
        let mut cleanup = valid.clone();
        let actions = &mut cleanup.modules[0].functions[0].cleanup_plans[0].actions;
        match mutation {
            0 => {
                actions.pop();
            }
            1 => actions.reverse(),
            2 => actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(0))),
            _ => unreachable!(),
        }
        reject(&authority, &valid, cleanup, "ZRYNA-I3012");
    }

    let mut active_cleanup = valid.clone();
    active_cleanup.modules[0].functions[0].blocks[3].instructions.remove(2);
    reject(&authority, &valid, active_cleanup, "ZRYNA-I3010");
}
