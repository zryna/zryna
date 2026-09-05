use super::opaque_handle_slots_fixture::Fixture;
use super::*;

#[test]
fn opaque_handle_slots_reuse_exact_composite_transfers_cleanup_and_operation_hooks() {
    let fixture = Fixture::new();
    let seed = fixture.seed();
    for _ in 0..2 {
        let verified = fixture.verify(seed.clone()).expect("typed handle slot composition");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[0].kind(), VerifiedInstructionKind::SharedClone);
        assert_eq!(instructions[1].kind(), VerifiedInstructionKind::WeakClone);
        for (index, expected) in [(0, vec![2, 1, 0]), (1, vec![3, 2, 1, 0]), (5, vec![7, 2, 1])] {
            assert_eq!(
                instructions[index]
                    .derived_drop_actions()
                    .map(|drop| drop.root().index())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert_eq!(instructions[2].kind(), VerifiedInstructionKind::StructConstruct);
        assert_eq!(instructions[3].kind(), VerifiedInstructionKind::EnumConstruct);
        assert_eq!(instructions[4].kind(), VerifiedInstructionKind::FixedArrayConstruct);
        assert_eq!(instructions[5].kind(), VerifiedInstructionKind::VecConstruct);
        let old = instructions[6].derived_drop_actions().next().expect("old vector drop");
        assert_eq!(old.root().index(), 2);
        assert_eq!(old.kind(), VerifiedDropActionKind::Place);
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|drop| drop.root().index())
                .collect::<Vec<_>>(),
            [1]
        );
        assert!(function.places().all(|place| !place.is_copy()));
    }
}

#[test]
fn opaque_handle_slots_reject_wrong_kind_duplicate_owner_and_premature_cleanup() {
    let fixture = Fixture::new();
    let seed = fixture.seed();
    for (case, code) in [(0, "ZRYNA-I3005"), (1, "ZRYNA-I3010"), (2, "ZRYNA-I3012")] {
        let mut raw = seed.clone();
        let function = &mut raw.modules[0].functions[0];
        if case < 2 {
            let raw::InstructionKind::StructConstruct { fields, .. } =
                &mut function.blocks[0].instructions[2].kind
            else {
                panic!("struct")
            };
            if case == 0 {
                fields.swap(0, 1);
            } else {
                fields[2] = fields[0];
            }
        } else {
            function.cleanup_plans[2].actions[0] = raw::DropAction::DropPlace(raw::PlaceId(8));
        }
        let first = fixture.verify(raw.clone()).expect_err("forged handle slot claim");
        assert!(first.iter().any(|error| error.code() == code), "case {case}: {first:?}");
        assert_eq!(first, fixture.verify(raw).expect_err("repeat rejection"));
        fixture.verify(seed.clone()).expect("valid recovery");
    }
}
