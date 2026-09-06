use super::active_enum_payload_borrow_fixture::{Case, fixture};
use super::*;

#[test]
fn refined_active_enum_payload_shared_and_exclusive_borrows_restore_parent() {
    for case in [Case::Shared, Case::Exclusive] {
        let (source, raw) = fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated enum payload borrow");
        let program = lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("{errors:?}\n{source}"));
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let begin = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("payload borrow begin");
        let end = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("payload borrow end");
        let borrowed = instructions[begin].place_operands().next().expect("payload place");
        let payload = function.places().find(|place| place.id() == borrowed).expect("payload");
        assert!(matches!(payload.kind(), VerifiedPlaceKind::EnumPayload { variant: 1, .. }));
        let operation = if matches!(case, Case::Exclusive) {
            VerifiedInstructionKind::BorrowReplace
        } else {
            VerifiedInstructionKind::GenericCloneBorrow
        };
        let operation = instructions
            .iter()
            .position(|instruction| instruction.kind() == operation)
            .expect("borrow operation");
        assert!(begin < operation && operation < end);
        assert!(
            instructions[end + 1..]
                .iter()
                .any(|instruction| instruction.kind() == VerifiedInstructionKind::MoveFromPlace),
            "parent owner is restored before its final transfer"
        );
        let replay = lower(pair_input(&syntax, &sources)).expect("deterministic valid replay");
        assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
    }
}

#[test]
fn refined_enum_payload_borrow_rejects_inactive_and_foreign_variants_then_recovers() {
    for case in [Case::Inactive, Case::Foreign] {
        let (source, raw) = fixture(case);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated hostile payload borrow");
        let first = lower(pair_input(&syntax, &sources)).expect_err("payload variant rejected");
        assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("deterministic replay"));
        assert!(first.iter().any(|error| error.code() == "ZRYNA-M3006"));
        let (valid_source, valid_raw) = fixture(Case::Shared);
        let valid_sources = sources_for(&valid_source);
        let valid_syntax =
            verify_snapshot(valid_raw, &valid_sources).expect("authenticated recovery payload");
        lower(pair_input(&valid_syntax, &valid_sources)).expect("recovery after payload rejection");
    }
}
