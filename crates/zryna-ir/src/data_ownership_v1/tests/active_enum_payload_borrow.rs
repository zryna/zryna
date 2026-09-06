pub(super) mod fixture;

use self::fixture::{Mode, Seed};
use super::*;

fn reject(seed: &Seed, valid: &raw::Program, hostile: raw::Program, code: &str) {
    let first = seed.check(hostile.clone()).expect_err("hostile enum payload borrow");
    assert!(first.iter().any(|error| error.code() == code), "expected {code}: {first:?}");
    assert_eq!(seed.check(hostile).expect_err("deterministic replay"), first);
    seed.check(valid.clone()).expect("same-authority recovery");
}

#[test]
fn active_enum_payload_borrow_seals_refinement_mode_replacement_and_parent_cleanup() {
    for mode in [Mode::Shared, Mode::Exclusive] {
        let seed = Seed::new(mode);
        let raw = seed.program();
        let verified = seed.check(raw.clone()).expect("active payload borrow");
        let function = verified.modules().next().expect("module").functions().next().expect("fn");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[0].kind(), VerifiedInstructionKind::EnumConstruct);
        assert_eq!(instructions[1].kind(), VerifiedInstructionKind::BeginBorrow);
        assert_eq!(instructions[1].place_operands().next().expect("payload").index(), 4);
        assert_eq!(instructions[1].borrow_access(), Some(mode.access().into()));
        assert_eq!(instructions.last().expect("end").kind(), VerifiedInstructionKind::EndBorrow);
        if mode == Mode::Exclusive {
            let replacement = instructions[2].borrow_replacement().expect("exclusive replacement");
            assert_eq!(replacement.borrow().index(), 0);
            assert_eq!(replacement.value().index(), 1);
            assert_eq!(replacement.referent().index(), seed.root.0);
            assert_eq!(replacement.old_value_drop().referent().index(), seed.root.0);
        }
        let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(cleanup[0].root().index(), 3);
        assert_eq!(cleanup[0].active_variant(), Some(1));
        assert!(cleanup[0].moved_projections().next().is_none());
        seed.check(raw).expect("valid deterministic replay");
    }
}

#[test]
fn enum_payload_borrow_rejects_inactive_foreign_type_and_mode_forgeries_then_recovers() {
    let seed = Seed::new(Mode::Exclusive);
    let valid = seed.program();

    let mut inactive = valid.clone();
    inactive.modules[0].functions[0].places[4].kind =
        raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 0 };
    reject(&seed, &valid, inactive, "ZRYNA-I3013");

    let mut foreign = valid.clone();
    foreign.modules[0].functions[0].places[4].kind =
        raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 1 };
    reject(&seed, &valid, foreign, "ZRYNA-I3006");

    let mut wrong_type = valid.clone();
    wrong_type.modules[0].functions[0].places[4].ty = seed.string;
    reject(&seed, &valid, wrong_type, "ZRYNA-I3006");

    let mut wrong_mode = valid.clone();
    let raw::InstructionKind::BeginBorrow(definition) =
        &mut wrong_mode.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        unreachable!()
    };
    definition.access = raw::BorrowAccess::Shared;
    reject(&seed, &valid, wrong_mode, "ZRYNA-I3005");
}

#[test]
fn enum_payload_borrow_rejects_moved_overlap_disjoint_and_ended_authority_then_recovers() {
    let seed = Seed::new(Mode::Exclusive);
    let valid = seed.program();
    let span = valid.modules[0].functions[0].span;

    let mut moved = valid.clone();
    moved.modules[0].functions[0].places.push(raw::Place {
        id: raw::PlaceId(5),
        ty: seed.root,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
    });
    moved.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(5)));
    moved.modules[0].functions[0].blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: seed.root, span }),
            span,
            kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) },
        },
    );
    reject(&seed, &valid, moved, "ZRYNA-I3011");

    let mut partial = valid.clone();
    partial.modules[0].functions[0].blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(4) },
        },
    );
    reject(&seed, &valid, partial, "ZRYNA-I3011");

    let mut overlap = valid.clone();
    overlap.modules[0].functions[0].blocks[0]
        .instructions
        .insert(2, begin_borrow(1, 3, raw::BorrowAccess::Shared, span));
    reject(&seed, &valid, overlap, "ZRYNA-I3011");

    let mut disjoint = valid.clone();
    disjoint.modules[0].functions[0].places.push(raw::Place {
        id: raw::PlaceId(5),
        ty: seed.root,
        span,
        kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 0 },
    });
    disjoint.modules[0].functions[0].blocks[0]
        .instructions
        .insert(2, begin_borrow(1, 5, raw::BorrowAccess::Shared, span));
    reject(&seed, &valid, disjoint, "ZRYNA-I3013");

    let mut ended = valid.clone();
    ended.modules[0].functions[0].blocks[0].instructions.swap(2, 3);
    reject(&seed, &valid, ended, "ZRYNA-I3011");
}
