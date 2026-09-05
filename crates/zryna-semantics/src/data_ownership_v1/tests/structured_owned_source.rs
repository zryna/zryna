use super::structured_owned_fixture::{Statement::*, fixture};
use super::*;

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
