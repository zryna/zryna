mod fixture;

use self::fixture::*;
use super::*;

#[test]
fn mixed_owner_nested_repeated_cfg_seals_transfers_variants_and_cleanup() {
    let authorities = authorities();
    let verified = verify_mixed(
        mixed_cfg(&authorities.0, &authorities.1, &authorities.2),
        &authorities.0,
        &authorities.1,
        &authorities.2,
    )
    .expect("mixed structured authority");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 14);
    assert_eq!(blocks[3].parameters().map(|value| value.id().index()).collect::<Vec<_>>(), [9, 10]);
    assert_eq!(blocks[4].parameters().map(|value| value.id().index()).collect::<Vec<_>>(), [11]);
    for index in [10, 12, 13] {
        let cleanup = blocks[index].terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [6, 5, 4, 3, 2, 8, 9]
        );
        assert_eq!(cleanup[2].active_variant(), Some(0));
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn mixed_cfg_rejects_state_mask_variant_owner_edge_and_cleanup_forgeries() {
    let authorities = authorities();
    let seed = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    let mut cases: Vec<(raw::Program, DiagnosticTrace)> = Vec::new();
    let mut uninitialized = seed.clone();
    uninitialized.modules[0].functions[0].places[STRUCT as usize].kind = raw::PlaceKind::Local(0);
    cases.push((uninitialized, trace(&[("ZRYNA-I3008", "non-Copy value has no root owner")])));

    let mut moved = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut moved.modules[0].functions[0].blocks[7].terminators[0].kind
    else {
        panic!("backedge")
    };
    edge.arguments[0] = raw::ValueId(JOIN_A);
    cases.push((
        moved,
        trace(&[(
            "ZRYNA-I3010",
            "partial non-Copy owner cannot enter a CFG edge without mask transfer",
        )]),
    ));

    let mut dropped = seed.clone();
    let span = dropped.modules[0].functions[0].span;
    dropped.modules[0].functions[0].blocks[7].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(STRUCT) },
    });
    cases.push((
        dropped,
        trace(&[(
            "ZRYNA-I3010",
            "ownership, initialization, or active-enum state differs across a CFG join or backedge",
        )]),
    ));

    let mut partial = seed.clone();
    let span = partial.modules[0].functions[0].span;
    partial.modules[0].functions[0].blocks[7].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(10) },
    });
    cases.push((
        partial,
        trace(&[(
            "ZRYNA-I3010",
            "ownership, initialization, or active-enum state differs across a CFG join or backedge",
        )]),
    ));

    let mut variant = seed.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut variant.modules[0].functions[0].blocks[6].terminators[0].kind
    else {
        panic!("match")
    };
    arms[0].variant = 1;
    cases.push((
        variant,
        trace(&[(
            "ZRYNA-I3014",
            "terminator result, condition, enum arms, or weak-upgrade edge shape is invalid",
        )]),
    ));

    let mut duplicate = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut duplicate.modules[0].functions[0].blocks[1].terminators[0].kind
    else {
        panic!("join")
    };
    edge.arguments[1] = raw::ValueId(STRING_A);
    cases.push((
        duplicate,
        trace(&[
            (
                "ZRYNA-I3010",
                "partial non-Copy owner cannot enter a CFG edge without mask transfer",
            ),
            (
                "ZRYNA-I3010",
                "ownership, initialization, or active-enum state differs across a CFG join or backedge",
            ),
            (
                "ZRYNA-I3012",
                "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            ),
            (
                "ZRYNA-I3012",
                "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            ),
            (
                "ZRYNA-I3012",
                "cleanup plan is incomplete, duplicated, or out of reverse-completion order",
            ),
        ]),
    ));

    let mut foreign_owner = seed.clone();
    foreign_owner.modules[0].functions[0].places[7].kind =
        raw::PlaceKind::Temporary(raw::ValueId(STRING_A));
    cases.push((
        foreign_owner,
        trace(&[
            ("ZRYNA-I3008", "non-Copy value has more than one root owner"),
            ("ZRYNA-I3008", "non-Copy value has no root owner"),
        ]),
    ));

    let mut arity = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut arity.modules[0].functions[0].blocks[1].terminators[0].kind
    else {
        panic!("join")
    };
    edge.arguments.pop();
    cases.push((
        arity,
        trace(&[("ZRYNA-I3007", "CFG edge argument arity does not match target block parameters")]),
    ));

    let mut edge_type = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut edge_type.modules[0].functions[0].blocks[1].terminators[0].kind
    else {
        panic!("join")
    };
    edge.arguments[0] = raw::ValueId(FLAG_A);
    cases.push((
        edge_type,
        trace(&[("ZRYNA-I3007", "CFG edge argument type does not match its target parameter")]),
    ));

    for (hostile, expected) in cases {
        reject_replay_recover(hostile, &expected, &authorities);
    }
}

#[test]
fn mixed_cfg_rejects_a_foreign_result_owner_identity() {
    let authorities = authorities();
    let mut hostile = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    hostile.modules[0].functions[0].places[7].kind = raw::PlaceKind::Temporary(raw::ValueId(99));
    reject_replay_recover(
        hostile,
        &trace(&[
            (
                "ZRYNA-I3006",
                "root place does not exactly match its parameter or temporary value type",
            ),
            ("ZRYNA-I3006", "place projection does not match its sealed layout"),
        ]),
        &authorities,
    );
}

#[test]
fn mixed_cfg_rejects_missing_extra_and_reordered_cleanup() {
    let authorities = authorities();
    let seed = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    for mutation in [0, 1, 2] {
        let mut cleanup = seed.clone();
        let actions = &mut cleanup.modules[0].functions[0].cleanup_plans[0].actions;
        let message = match mutation {
            0 => {
                actions.pop();
                "cleanup plan is incomplete, duplicated, or out of reverse-completion order"
            }
            1 => {
                actions.push(raw::DropAction::DropPlace(raw::PlaceId(9)));
                "cleanup plan has a noncanonical identity or foreign place"
            }
            _ => {
                actions.swap(0, 1);
                "cleanup plan is incomplete, duplicated, or out of reverse-completion order"
            }
        };
        reject_replay_recover(cleanup, &trace(&[("ZRYNA-I3012", message)]), &authorities);
    }
}

#[test]
fn mixed_cfg_rejects_lexical_borrow_escape_at_every_edge_and_exit() {
    let authorities = authorities();
    for block_index in [0, 1, 7, 10, 13] {
        let mut hostile = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
        let span = hostile.modules[0].functions[0].span;
        hostile.modules[0].functions[0].blocks[block_index].instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(STRUCT),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        });
        reject_replay_recover(
            hostile,
            &trace(&[("ZRYNA-I3011", "borrow remains active at a control-flow edge")]),
            &authorities,
        );
    }
}

#[test]
fn mixed_cfg_join_diagnostic_is_independent_of_branch_target_order() {
    let authorities = authorities();
    let mut forward = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    let span = forward.modules[0].functions[0].span;
    forward.modules[0].functions[0].blocks[7].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(10) },
    });
    let mut reversed = forward.clone();
    let raw::Terminator::Branch { when_true, when_false, .. } =
        &mut reversed.modules[0].functions[0].blocks[5].terminators[0].kind
    else {
        panic!("nested branch")
    };
    std::mem::swap(when_true, when_false);
    let forward = diagnostic_trace(
        verify_mixed(forward, &authorities.0, &authorities.1, &authorities.2)
            .expect_err("partial backedge"),
    );
    let reversed = diagnostic_trace(
        verify_mixed(reversed, &authorities.0, &authorities.1, &authorities.2)
            .expect_err("reordered partial backedge"),
    );
    let expected = trace(&[(
        "ZRYNA-I3010",
        "ownership, initialization, or active-enum state differs across a CFG join or backedge",
    )]);
    assert_eq!(forward, expected);
    assert_eq!(reversed, expected);
}
