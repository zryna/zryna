use super::generic_vec_fixture::structured_payload_cfg_fixture::{Payload, fixture};
use super::*;

#[test]
fn payload_matrix_composes_handles_vec_calls_upgrade_and_nested_cfg() {
    for payload in [
        Payload::String,
        Payload::Struct,
        Payload::Enum,
        Payload::FixedArray,
        Payload::Vec,
        Payload::NestedShared,
        Payload::NestedWeak,
        Payload::RecursiveVec,
    ] {
        let (source, raw) = fixture(payload);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated CFG payload fixture");
        let first = lower(pair_input(&syntax, &sources)).expect("payload CFG lowering");
        let function = first
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .nth(1)
            .expect("composition function");
        let blocks = function.blocks().collect::<Vec<_>>();
        let kinds = blocks
            .iter()
            .flat_map(|block| block.instructions())
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
            .collect::<Vec<_>>();
        for expected in [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::DirectCall,
        ] {
            assert!(kinds.contains(&expected), "{payload:?} misses {expected:?}");
        }
        assert_eq!(
            blocks
                .iter()
                .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::Return)
                .count(),
            2,
            "early and post-loop returns"
        );
        let (success, expired) = blocks
            .iter()
            .find_map(|block| block.terminator().weak_upgrade_edges())
            .expect("WeakUpgrade occupant");
        assert_eq!(
            blocks
                .iter()
                .find(|block| block.id() == success.target())
                .expect("upgrade success")
                .parameters()
                .count(),
            1
        );
        assert_eq!(
            blocks
                .iter()
                .find(|block| block.id() == expired.target())
                .expect("upgrade expiration")
                .parameters()
                .count(),
            0
        );
        assert!(blocks.iter().enumerate().any(|(index, block)| {
            block
                .terminator()
                .edges()
                .any(|edge| edge.target().index() <= u32::try_from(index).expect("block index"))
        }));
        let second = lower(pair_input(&syntax, &sources)).expect("deterministic payload replay");
        assert_eq!(first.verified_ir().identity(), second.verified_ir().identity());
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn payload_cfg_fallible_operations_keep_source_ordered_cleanup() {
    let (source, raw) = fixture(Payload::RecursiveVec);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated recursive CFG payload");
    let program = lower(pair_input(&syntax, &sources)).expect("recursive payload CFG");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .nth(1)
        .expect("composition function");
    let fallible = function
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .filter(|instruction| {
            matches!(
                instruction.kind(),
                VerifiedInstructionKind::SharedConstruct
                    | VerifiedInstructionKind::SharedClone
                    | VerifiedInstructionKind::WeakDowngrade
                    | VerifiedInstructionKind::WeakClone
                    | VerifiedInstructionKind::VecConstruct
                    | VerifiedInstructionKind::DirectCall
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fallible.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
            VerifiedInstructionKind::DirectCall,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::WeakClone,
        ],
        "fallible occupants retain exact source order"
    );
    let fault_cleanup = fallible
        .iter()
        .map(|instruction| instruction.derived_drop_actions().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        fault_cleanup
            .iter()
            .map(|actions| actions.iter().map(|action| action.root().index()).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [vec![2], vec![4], vec![6, 4], vec![6, 4], vec![9, 6, 4], vec![10, 9, 6, 4], vec![6, 4]],
        "each fault retains its source and reverse-cleans the exact completed owner prefix"
    );
    assert!(fault_cleanup.iter().flatten().all(|action| {
        action.kind() == VerifiedDropActionKind::Place
            && action.moved_projections().next().is_none()
            && action.initialized_projections().next().is_none()
            && action.active_variant().is_none()
    }));
    let nested_scope = function
        .blocks()
        .find(|block| {
            block
                .instructions()
                .any(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        })
        .expect("nested call and Vec scope");
    let nested_kinds = nested_scope
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert!(
        nested_kinds
            .ends_with(&[VerifiedInstructionKind::DropPlace, VerifiedInstructionKind::DropPlace,]),
        "nested call and Vec owners fall through in reverse order"
    );
    let loop_local = function
        .blocks()
        .find(|block| {
            let kinds = block
                .instructions()
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>();
            kinds.contains(&VerifiedInstructionKind::WeakClone)
                && !kinds.contains(&VerifiedInstructionKind::DirectCall)
        })
        .expect("loop-local Weak clone");
    assert_eq!(
        loop_local
            .instructions()
            .last()
            .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind),
        Some(VerifiedInstructionKind::DropPlace),
        "loop-local Weak clone is dropped before control rejoins the backedge"
    );
    let returns = function
        .blocks()
        .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::Return)
        .map(|block| {
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let upgrade_cleanup = function
        .blocks()
        .find_map(|block| {
            block.terminator().weak_upgrade_edges().map(|_| {
                block
                    .terminator()
                    .derived_drop_actions()
                    .map(|action| {
                        (
                            action.root().index(),
                            action.kind(),
                            action
                                .moved_projections()
                                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                                .collect::<Vec<_>>(),
                            action
                                .initialized_projections()
                                .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                                .collect::<Vec<_>>(),
                            action.active_variant(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
        })
        .expect("WeakUpgrade cleanup");
    assert_eq!(
        upgrade_cleanup,
        [
            (6, VerifiedDropActionKind::Place, vec![], vec![], None),
            (4, VerifiedDropActionKind::Place, vec![], vec![], None),
        ],
        "upgrade refcount fault reverse-cleans only live owners"
    );
    assert_eq!(returns, [vec![6], vec![6]], "early and false-exit cleanup agree exactly");
}
