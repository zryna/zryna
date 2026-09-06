use super::structured_owned_fixture::{
    Payload, Statement, Statement::*, fixture, legacy_payload_fixture,
};
use super::*;

fn assert_replays(statements: &[Statement]) {
    let (text, raw) = fixture(statements);
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated structured source");
    let first = lower(pair_input(&syntax, &sources)).expect("structured ownership verifies");
    let second = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
    assert_eq!(format!("{:?}", first.verified_ir()), format!("{:?}", second.verified_ir()));
}

#[test]
fn structured_owned_nested_repeated_branches_and_loops_verify_without_owner_repair() {
    let cases = [
        vec![
            If(
                vec![Local("first", "seed", true), If(vec![Local("inner", "seed", true)], vec![])],
                vec![],
            ),
            If(vec![], vec![Local("last", "seed", true)]),
            Return("seed"),
        ],
        vec![While(vec![If(vec![Local("inner", "seed", true)], vec![])]), Return("seed")],
        vec![If(vec![Return("seed")], vec![Return("seed")])],
    ];
    for statements in cases {
        let (text, raw) = fixture(&statements);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated structured source");
        let program = lower(pair_input(&syntax, &sources)).expect("structured ownership verifies");
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function");
        assert!(function.blocks().count() >= 3);
        let repeated = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
        assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", repeated.verified_ir()));
    }
}

#[test]
fn structured_owned_unequal_branches_and_loop_header_moves_reject_deterministically() {
    let cases = [
        vec![If(vec![Local("moved", "seed", false)], vec![]), Return("seed")],
        vec![While(vec![Local("moved", "seed", false)]), Return("seed")],
    ];
    for statements in cases {
        let (text, raw) = fixture(&statements);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated unequal state");
        let first = lower(pair_input(&syntax, &sources)).expect_err("unequal ownership rejects");
        let second = lower(pair_input(&syntax, &sources)).expect_err("same rejection");
        assert_eq!(first, second);
        assert_eq!(first[0].code(), "ZRYNA-M3015");
    }
}

#[test]
fn structured_owned_mixed_graphs_compose_nested_scopes_loops_and_returns() {
    use structured_owned_fixture::payload_fixture;
    for payload in [
        Payload::Array(0),
        Payload::Array(2),
        Payload::Vec,
        Payload::Nested,
        Payload::Struct,
        Payload::Enum,
    ] {
        let body = [
            While(vec![If(vec![Local("inner", "seed", true)], vec![])]),
            If(vec![Local("last", "seed", true), Return("last")], vec![Return("seed")]),
        ];
        let (text, raw) = payload_fixture(&body, payload);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed graph CFG");
        let program = lower(pair_input(&syntax, &sources)).expect("mixed ownership flow verifies");
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function");
        assert_eq!(
            function
                .blocks()
                .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::Return)
                .count(),
            2
        );
    }
}

#[test]
fn legacy_string_and_vec_signatures_route_from_shared_structured_shapes() {
    for (payload, body) in [
        (Payload::String, vec![Block(vec![Local("copy", "seed", true)]), Return("seed")]),
        (Payload::Vec, vec![IfWithoutElse(vec![Local("copy", "seed", true)]), Return("seed")]),
    ] {
        let (text, raw) = legacy_payload_fixture(&body, payload);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated legacy signature");
        let first = lower(pair_input(&syntax, &sources)).expect("shared structured route");
        let second = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
        assert_eq!(format!("{:?}", first.verified_ir()), format!("{:?}", second.verified_ir()));
    }
}

#[test]
fn lexical_shadowing_drops_the_inner_owner_and_restores_the_outer_binding() {
    let body = [Block(vec![Local("seed", "seed", true)]), Return("seed")];
    let (text, raw) = fixture(&body);
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated lexical shadow");
    let program = lower(pair_input(&syntax, &sources)).expect("outer binding restored");
    let kinds = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::DropPlace).count(), 1);
    assert_replays(&body);
}

#[test]
fn omitted_else_asymmetric_return_loop_return_and_post_loop_are_reachable() {
    let cases = [
        vec![IfWithoutElse(vec![Local("copy", "seed", true)]), Return("seed")],
        vec![If(vec![Return("seed")], vec![Local("copy", "seed", true)]), Return("seed")],
        vec![While(vec![Return("seed")]), Return("seed")],
        vec![
            While(vec![Local("inside", "seed", true)]),
            Local("after", "seed", true),
            Return("after"),
        ],
    ];
    for body in cases {
        assert_replays(&body);
    }
}

#[test]
fn mutable_copy_statement_updates_a_loop_condition_exactly_once() {
    let body = [
        BoolLocal("run", true, true),
        WhileOn("run", vec![AssignBool("run", false)]),
        Return("seed"),
    ];
    let (text, raw) = fixture(&body);
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mutable copy statement");
    let program = lower(pair_input(&syntax, &sources)).expect("copy assignment verifies");
    let kinds = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::ReplacePlace).count(),
        1
    );
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::BoolLiteral).count(),
        2
    );
    assert_eq!(
        kinds.iter().filter(|kind| **kind == VerifiedInstructionKind::CopyFromPlace).count(),
        1
    );
    assert_replays(&body);
}

#[test]
fn structured_copy_mutability_type_and_unreachable_errors_replay_without_repair() {
    let cases = [
        (
            vec![
                Block(vec![BoolLocal("run", false, true), AssignBool("run", false)]),
                Return("seed"),
            ],
            "ZRYNA-M3013",
        ),
        (
            vec![
                Block(vec![BoolLocal("run", true, true), AssignReference("run", "seed")]),
                Return("seed"),
            ],
            "ZRYNA-M3016",
        ),
        (vec![If(vec![Return("seed")], vec![Return("seed")]), Return("seed")], "ZRYNA-M3015"),
    ];
    for (body, expected_code) in cases {
        let (text, raw) = fixture(&body);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated rejected source");
        let first = lower(pair_input(&syntax, &sources)).expect_err("structured rejection");
        let second = lower(pair_input(&syntax, &sources)).expect_err("same rejection");
        assert_eq!(first, second);
        assert_eq!(first[0].code(), expected_code);
    }
}

#[test]
fn lexical_shadowing_does_not_admit_a_same_block_duplicate() {
    let body =
        [Block(vec![Local("copy", "seed", true), Local("copy", "seed", true)]), Return("seed")];
    let (text, raw) = fixture(&body);
    let sources = sources_for(&text);
    let diagnostics = verify_snapshot(raw, &sources).expect_err("same-scope duplicate rejects");
    assert_eq!(diagnostics[0].code(), "ZRYNA-Y4002");
}
