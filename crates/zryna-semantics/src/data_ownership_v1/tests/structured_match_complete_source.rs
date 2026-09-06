use super::*;

#[test]
fn structured_match_exhausts_payloadless_and_mixed_payload_variants() {
    let (text, raw) = structured_owned_fixture::mixed_variant_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed-variant match");
    let program = lower(pair_input(&syntax, &sources)).expect("mixed-variant match verifies");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 5, "entry, three arms, and one result continuation");
    assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
    assert_eq!(blocks[0].terminator().edges().len(), 3);
    assert_eq!(blocks[4].parameters().len(), 1);
    assert_eq!(
        blocks[1].instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::CopyFromPlace, VerifiedInstructionKind::DropPlace],
        "payloadless arm reads only the fallback before dropping its active empty enum"
    );
    assert_eq!(
        blocks[2].instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::CopyFromPlace, VerifiedInstructionKind::DropPlace],
        "unused String payload is cleaned only through the refined enum root"
    );
    assert_eq!(
        blocks[3].instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::CopyFromPlace, VerifiedInstructionKind::DropPlace],
        "active Copy payload is read through its exact refined projection"
    );
    for (ordinal, arm) in blocks[1..4].iter().enumerate() {
        let drop = arm.instructions().last().expect("arm enum cleanup");
        let actions = drop.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].active_variant(),
            Some(u32::try_from(ordinal).expect("three variants"))
        );
        assert_eq!(arm.terminator().edges().next().expect("continuation").arguments().len(), 1);
    }
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic mixed match replay");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
}

#[test]
fn structured_empty_match_rejects_with_one_stable_exhaustiveness_diagnostic() {
    let (text, mut raw) = structured_owned_fixture::mixed_variant_fixture();
    let function = &mut raw.files[0].functions[0];
    let match_index = function
        .body
        .expressions
        .iter()
        .position(|expression| {
            matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
        })
        .expect("match expression");
    let mut matching = function.body.expressions[match_index].clone();
    let at = matching.span;
    let zryna_syntax::v4::RawExpressionKind::Match { scrutinee, arms, .. } = &mut matching.kind
    else {
        unreachable!("selected match expression")
    };
    let source = function.body.expressions[*scrutinee as usize].clone();
    *scrutinee = 0;
    arms.clear();
    function.body.expressions = vec![source, matching];
    let RawStatementKind::Return { value, .. } = &mut function.body.statements[0].kind else {
        unreachable!("fixture return")
    };
    *value = 1;
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated empty match graph");
    let first = lower(pair_input(&syntax, &sources)).expect_err("empty match rejected");
    assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("deterministic replay"));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3009");
    assert_eq!(first[0].primary_span(), Some(span(&sources, at)));
    assert_eq!(first[0].message(), "match does not cover any enum variant");
    assert_eq!(first[0].guidance(), "provide one arm for every declared variant");
}

#[test]
fn structured_nested_matches_transfer_only_each_active_payload() {
    let (text, raw) = structured_owned_fixture::nested_variant_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated nested matches");
    let program = lower(pair_input(&syntax, &sources)).expect("nested matches verify");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block.terminator().kind() == VerifiedTerminatorKind::EnumMatch)
            .count(),
        3,
        "one outer match and one nested match in each reachable outer arm"
    );
    assert_eq!(
        blocks.iter().filter(|block| block.parameters().len() == 1).count(),
        3,
        "each inner result and the outer result have one exact owned continuation owner"
    );
    let moves = blocks
        .iter()
        .flat_map(|block| block.instructions())
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::GenericMoveFromPlace)
        .count();
    assert_eq!(moves, 6, "two outer payload moves and four active String payload moves");
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic nested match replay");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
}
