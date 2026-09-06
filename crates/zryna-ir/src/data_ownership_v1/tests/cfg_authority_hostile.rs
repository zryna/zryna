use super::*;

const STRING_A: u32 = 0;
const STRING_B: u32 = 1;
const VEC: u32 = 2;
const STRUCT: u32 = 3;
const ENUM: u32 = 4;
const SHARED: u32 = 5;
const WEAK: u32 = 6;
const FLAG_A: u32 = 7;
const FLAG_B: u32 = 8;
const JOIN_A: u32 = 9;
const JOIN_B: u32 = 10;
const HEADER_A: u32 = 11;

fn edge(target: u32, arguments: Vec<u32>) -> raw::Edge {
    raw::Edge {
        target: raw::BlockId(target),
        arguments: arguments.into_iter().map(raw::ValueId).collect(),
    }
}

fn terminator(span: zryna_source::Span, kind: raw::Terminator) -> raw::SpannedTerminator {
    raw::SpannedTerminator { span, kind }
}

fn block(id: u32, kind: raw::Terminator, span: zryna_source::Span) -> raw::Block {
    raw::Block {
        id: raw::BlockId(id),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: vec![terminator(span, kind)],
    }
}

fn mixed_blocks(span: zryna_source::Span) -> Vec<raw::Block> {
    let mut blocks = vec![
        block(
            0,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(1, vec![]),
                when_false: edge(2, vec![]),
            },
            span,
        ),
        block(1, raw::Terminator::Jump(edge(3, vec![STRING_A, STRING_B])), span),
        block(2, raw::Terminator::Jump(edge(3, vec![STRING_A, STRING_B])), span),
        block(3, raw::Terminator::Jump(edge(4, vec![JOIN_A])), span),
        block(
            4,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(5, vec![]),
                when_false: edge(6, vec![]),
            },
            span,
        ),
        block(
            5,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_B),
                when_true: edge(7, vec![]),
                when_false: edge(8, vec![]),
            },
            span,
        ),
        block(
            6,
            raw::Terminator::EnumMatch {
                place: raw::PlaceId(ENUM),
                arms: vec![raw::EnumArm { variant: 0, edge: edge(9, vec![]) }],
            },
            span,
        ),
        block(7, raw::Terminator::Jump(edge(4, vec![HEADER_A])), span),
        block(8, raw::Terminator::Jump(edge(4, vec![HEADER_A])), span),
        block(
            9,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(10, vec![]),
                when_false: edge(11, vec![]),
            },
            span,
        ),
        block(
            10,
            raw::Terminator::Return { value: raw::ValueId(FLAG_A), cleanup: raw::CleanupPlanId(0) },
            span,
        ),
        block(
            11,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_B),
                when_true: edge(12, vec![]),
                when_false: edge(13, vec![]),
            },
            span,
        ),
        block(
            12,
            raw::Terminator::Return { value: raw::ValueId(FLAG_B), cleanup: raw::CleanupPlanId(1) },
            span,
        ),
        block(
            13,
            raw::Terminator::Trap {
                identity: raw::TrapIdentity::BoundsV1,
                cleanup: raw::CleanupPlanId(2),
            },
            span,
        ),
    ];
    blocks[3].parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(JOIN_A), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(JOIN_B), ty: raw::TypeId(2), span },
    ];
    blocks[4].parameters =
        vec![raw::ValueDefinition { id: raw::ValueId(HEADER_A), ty: raw::TypeId(2), span }];
    blocks
}

fn mixed_cfg(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = [
        (STRING_A, 2),
        (STRING_B, 2),
        (VEC, 5),
        (STRUCT, 3),
        (ENUM, 4),
        (SHARED, 6),
        (WEAK, 7),
        (FLAG_A, 0),
        (FLAG_B, 0),
    ]
    .into_iter()
    .map(|(id, ty)| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(ty), span })
    .collect();
    function.result = raw::TypeId(0);
    function.places = [
        (0, 2, raw::PlaceKind::Parameter(STRING_A)),
        (1, 2, raw::PlaceKind::Parameter(STRING_B)),
        (2, 5, raw::PlaceKind::Parameter(VEC)),
        (3, 3, raw::PlaceKind::Parameter(STRUCT)),
        (4, 4, raw::PlaceKind::Parameter(ENUM)),
        (5, 6, raw::PlaceKind::Parameter(SHARED)),
        (6, 7, raw::PlaceKind::Parameter(WEAK)),
        (7, 2, raw::PlaceKind::Temporary(raw::ValueId(JOIN_A))),
        (8, 2, raw::PlaceKind::Temporary(raw::ValueId(JOIN_B))),
        (9, 2, raw::PlaceKind::Temporary(raw::ValueId(HEADER_A))),
        (10, 2, raw::PlaceKind::StructField { base: raw::PlaceId(STRUCT), ordinal: 0 }),
    ]
    .into_iter()
    .map(|(id, ty, kind)| raw::Place { id: raw::PlaceId(id), ty: raw::TypeId(ty), span, kind })
    .collect();

    function.blocks = mixed_blocks(span);
    let actions = [6, 5, 4, 3, 2, 8, 9]
        .into_iter()
        .map(|id| raw::DropAction::DropPlace(raw::PlaceId(id)))
        .collect::<Vec<_>>();
    function.cleanup_plans = (0..3)
        .map(|id| raw::CleanupPlan { id: raw::CleanupPlanId(id), span, actions: actions.clone() })
        .collect();
    raw
}

fn verify_mixed(
    raw: raw::Program,
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> Result<super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
    verify(raw, sources, sources.verify_file_id(0).expect("entry"), linear.clone(), linux.clone())
}

fn reject_replay_recover(
    hostile: raw::Program,
    code: &str,
    message: &str,
    authorities: &(SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts),
) -> DiagnosticTrace {
    let (sources, linear, linux) = authorities;
    let first = diagnostic_trace(
        verify_mixed(hostile.clone(), sources, linear, linux).expect_err("hostile IR must reject"),
    );
    let second = diagnostic_trace(
        verify_mixed(hostile, sources, linear, linux).expect_err("hostile replay must reject"),
    );
    assert_eq!(first, second);
    assert!(
        first.iter().any(|(actual_code, actual_message, _)| {
            actual_code == code && actual_message == message
        }),
        "missing {code}/{message}: {first:?}"
    );
    verify_mixed(mixed_cfg(sources, linear, linux), sources, linear, linux)
        .expect("valid authority recovers after rejection");
    first
}

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
fn mixed_cfg_rejects_state_mask_variant_owner_edge_and_cleanup_forgeries() {
    let authorities = authorities();
    let seed = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    let mut cases: Vec<(raw::Program, &str, &str)> = Vec::new();
    let mut uninitialized = seed.clone();
    uninitialized.modules[0].functions[0].places[STRUCT as usize].kind = raw::PlaceKind::Local(0);
    cases.push((uninitialized, "ZRYNA-I3008", "non-Copy value has no root owner"));

    let mut moved = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut moved.modules[0].functions[0].blocks[7].terminators[0].kind
    else {
        panic!("backedge")
    };
    edge.arguments[0] = raw::ValueId(JOIN_A);
    cases.push((
        moved,
        "ZRYNA-I3010",
        "partial non-Copy owner cannot enter a CFG edge without mask transfer",
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
        "ZRYNA-I3010",
        "ownership, initialization, or active-enum state differs across a CFG join or backedge",
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
        "ZRYNA-I3010",
        "ownership, initialization, or active-enum state differs across a CFG join or backedge",
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
        "ZRYNA-I3014",
        "terminator result, condition, enum arms, or weak-upgrade edge shape is invalid",
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
        "ZRYNA-I3010",
        "partial non-Copy owner cannot enter a CFG edge without mask transfer",
    ));

    let mut foreign_owner = seed.clone();
    foreign_owner.modules[0].functions[0].places[7].kind =
        raw::PlaceKind::Temporary(raw::ValueId(STRING_A));
    cases.push((foreign_owner, "ZRYNA-I3008", "non-Copy value has more than one root owner"));

    let mut arity = seed.clone();
    let raw::Terminator::Jump(edge) =
        &mut arity.modules[0].functions[0].blocks[1].terminators[0].kind
    else {
        panic!("join")
    };
    edge.arguments.pop();
    cases.push((
        arity,
        "ZRYNA-I3007",
        "CFG edge argument arity does not match target block parameters",
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
        "ZRYNA-I3007",
        "CFG edge argument type does not match its target parameter",
    ));

    for (hostile, code, message) in cases {
        reject_replay_recover(hostile, code, message, &authorities);
    }
}

#[test]
fn mixed_cfg_rejects_a_foreign_result_owner_identity() {
    let authorities = authorities();
    let mut hostile = mixed_cfg(&authorities.0, &authorities.1, &authorities.2);
    hostile.modules[0].functions[0].places[7].kind = raw::PlaceKind::Temporary(raw::ValueId(99));
    reject_replay_recover(
        hostile,
        "ZRYNA-I3006",
        "root place does not exactly match its parameter or temporary value type",
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
        reject_replay_recover(cleanup, "ZRYNA-I3012", message, &authorities);
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
            "ZRYNA-I3011",
            "borrow remains active at a control-flow edge",
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
    assert_eq!(forward, reversed);
}
