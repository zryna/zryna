use super::enum_match_payload_borrow_fixture::{MatchBorrowCase, match_borrow_fixture};
use super::*;

#[test]
fn exhaustive_match_arms_borrow_only_the_refined_active_owned_payload() {
    for case in [MatchBorrowCase::Shared, MatchBorrowCase::Exclusive] {
        let (source, raw) = match_borrow_fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated match payload borrow");
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
        let payload_arm = &blocks[2];
        let instructions = payload_arm.instructions().collect::<Vec<_>>();
        let begin = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("begin");
        let clone = instructions
            .iter()
            .position(|instruction| {
                instruction.kind() == VerifiedInstructionKind::GenericCloneBorrow
            })
            .expect("clone");
        let end = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("end");
        assert!(begin < clone && clone < end);
        let payload = instructions[begin].place_operands().next().expect("refined payload");
        let place = function.places().find(|place| place.id() == payload).expect("payload place");
        assert!(matches!(place.kind(), VerifiedPlaceKind::EnumPayload { variant: 1, .. }));
        assert_eq!(
            instructions[begin].borrow_access(),
            Some(if matches!(case, MatchBorrowCase::Exclusive) {
                VerifiedBorrowAccess::Exclusive
            } else {
                VerifiedBorrowAccess::Shared
            })
        );
        let cleanup = instructions[clone].derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(cleanup.last().expect("refined root cleanup").active_variant(), Some(1));
    }
}

#[test]
fn inactive_match_payload_borrow_rejects_deterministically_then_recovers() {
    let (source, raw) = match_borrow_fixture(MatchBorrowCase::Inactive);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated inactive payload source");
    let first = lower(pair_input(&syntax, &sources)).expect_err("inactive payload rejected");
    assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("replay"));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3017");
    assert_eq!(
        first[0].message(),
        "inline refined borrowing requires one active match payload binding"
    );

    let (valid_source, valid_raw) = match_borrow_fixture(MatchBorrowCase::Shared);
    let valid_sources = sources_for(&valid_source);
    let valid_syntax = verify_snapshot(valid_raw, &valid_sources).expect("valid recovery source");
    lower(pair_input(&valid_syntax, &valid_sources)).expect("recovery after inactive payload");
}
