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
        let call = blocks
            .iter()
            .flat_map(|block| block.instructions())
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .expect("mixed owned call in nested loop scope");
        let arguments = call.call_arguments().collect::<Vec<_>>();
        assert_eq!(arguments.len(), 3, "{payload:?} exact mixed call arity");
        assert!(
            arguments.iter().all(|argument| matches!(argument, VerifiedCallArgument::Value(_))),
            "{payload:?} keeps three by-value arguments"
        );
        assert_eq!(
            call.derived_drop_actions().len(),
            2,
            "{payload:?} CallTrap excludes both transferred owned arguments"
        );
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
    let module = program.verified_ir().modules().next().expect("module");
    let functions = module.functions().collect::<Vec<_>>();
    let callee = functions[0];
    let function = functions[1];
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
            VerifiedInstructionKind::SharedClone,
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
    let fault_signatures = fault_cleanup
        .iter()
        .map(|actions| {
            actions
                .iter()
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
        .collect::<Vec<_>>();
    let place = VerifiedDropActionKind::Place;
    assert_eq!(
        fault_signatures,
        [
            vec![(2, place, vec![], vec![], None)],
            vec![(4, place, vec![], vec![], None)],
            vec![(6, place, vec![], vec![], None), (4, place, vec![], vec![], None)],
            vec![
                (7, place, vec![], vec![], None),
                (6, place, vec![], vec![], None),
                (4, place, vec![], vec![], None),
            ],
            vec![(6, place, vec![], vec![], None), (4, place, vec![], vec![], None)],
            vec![
                (10, place, vec![], vec![], None),
                (6, place, vec![], vec![], None),
                (4, place, vec![], vec![], None),
            ],
            vec![
                (11, place, vec![], vec![], None),
                (10, place, vec![], vec![], None),
                (6, place, vec![], vec![], None),
                (4, place, vec![], vec![], None),
            ],
            vec![(6, place, vec![], vec![], None), (4, place, vec![], vec![], None)],
        ],
        "each fault retains its source and reverse-cleans the exact completed owner prefix"
    );
    let discarded = callee
        .places()
        .find(|place| place.kind() == VerifiedPlaceKind::Parameter(2))
        .expect("discarded owned parameter");
    assert_eq!(
        callee
            .blocks()
            .next()
            .expect("callee entry")
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [discarded.id()],
        "callee owns and drops the unreturned transferred input"
    );
    let nested_scope = function
        .blocks()
        .find(|block| {
            block
                .instructions()
                .any(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
        })
        .expect("nested call and Vec scope");
    assert_eq!(nested_scope.id().index(), 2, "nested scope stays in the loop body");
    let nested_instructions = nested_scope
        .instructions()
        .map(|instruction| {
            (
                instruction.kind(),
                instruction
                    .place_operands()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        nested_instructions,
        [
            (VerifiedInstructionKind::WeakClone, vec![6]),
            (VerifiedInstructionKind::CopyFromPlace, vec![1]),
            (VerifiedInstructionKind::SharedClone, vec![4]),
            (VerifiedInstructionKind::DirectCall, vec![]),
            (VerifiedInstructionKind::InitializePlace, vec![10]),
            (VerifiedInstructionKind::SharedClone, vec![4]),
            (VerifiedInstructionKind::VecConstruct, vec![]),
            (VerifiedInstructionKind::InitializePlace, vec![13]),
            (VerifiedInstructionKind::DropPlace, vec![13]),
            (VerifiedInstructionKind::DropPlace, vec![10]),
        ],
        "nested call and Vec owners fall through in exact reverse place order"
    );
    assert_eq!(
        nested_scope.terminator().kind(),
        VerifiedTerminatorKind::WeakUpgradeBranch,
        "fallthrough cleanup precedes the loop body's upgrade branch"
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
    assert_eq!(loop_local.id().index(), 7, "false arm remains the loop-local clone block");
    assert_eq!(
        loop_local
            .instructions()
            .map(|instruction| (
                instruction.kind(),
                instruction
                    .place_operands()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>(),
        [
            (VerifiedInstructionKind::WeakClone, vec![6]),
            (VerifiedInstructionKind::InitializePlace, vec![17]),
            (VerifiedInstructionKind::DropPlace, vec![17]),
        ],
        "loop-local Weak clone is dropped by exact place before control rejoins"
    );
    assert_eq!(loop_local.terminator().kind(), VerifiedTerminatorKind::Jump);
    assert_eq!(
        loop_local.terminator().edges().map(|edge| edge.target().index()).collect::<Vec<_>>(),
        [8],
        "loop-local fallthrough enters the dedicated backedge block"
    );
    let backedge = function.blocks().find(|block| block.id().index() == 8).expect("backedge block");
    assert_eq!(backedge.instructions().count(), 0);
    assert_eq!(backedge.terminator().kind(), VerifiedTerminatorKind::Jump);
    assert_eq!(
        backedge.terminator().edges().map(|edge| edge.target().index()).collect::<Vec<_>>(),
        [1],
        "backedge returns to the fixed loop header"
    );
    let returns = function
        .blocks()
        .map(|block| {
            (
                block.id().index(),
                block.terminator().kind(),
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
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, kind, _)| *kind == VerifiedTerminatorKind::Return)
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
    assert_eq!(
        returns,
        [
            (6, VerifiedTerminatorKind::Return, vec![(6, place, vec![], vec![], None)]),
            (9, VerifiedTerminatorKind::Return, vec![(6, place, vec![], vec![], None)]),
        ],
        "early and false-exit returns retain exact full cleanup identities"
    );
}
