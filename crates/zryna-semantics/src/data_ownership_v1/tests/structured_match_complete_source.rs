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
fn structured_match_arm_order_is_canonical_and_scrutinee_is_consumed_once() {
    let (text, raw) = structured_owned_fixture::reordered_mixed_variant_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated reordered match");
    let program = lower(pair_input(&syntax, &sources)).expect("reordered match verifies");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let entry = function.blocks().next().expect("entry");
    assert_eq!(
        entry.terminator().enum_arms().map(|arm| arm.variant()).collect::<Vec<_>>(),
        [0, 1, 2],
        "source arm order is normalized to sealed declaration order"
    );
    let scrutinee = entry.terminator().place_operands().next().expect("match scrutinee place");
    assert_eq!(
        entry.terminator().place_operands().collect::<Vec<_>>(),
        [scrutinee],
        "the match consumes one materialized scrutinee"
    );
    assert_eq!(
        function
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
            .count(),
        1,
        "the fallible constructed scrutinee is evaluated exactly once before dispatch"
    );
}

#[test]
fn structured_match_invalid_arm_sets_reject_exactly_replay_and_recover() {
    use structured_owned_fixture::InvalidMatch;

    let cases = [
        (
            InvalidMatch::Empty,
            None,
            "match does not cover any enum variant",
            "provide one arm for every declared variant",
        ),
        (
            InvalidMatch::Missing,
            None,
            "match does not cover every enum variant exactly once",
            "provide one arm for every declared variant",
        ),
        (
            InvalidMatch::Duplicate,
            None,
            "duplicate qualified match arm",
            "return source-faithful canonical protocol-v4 syntax",
        ),
        (
            InvalidMatch::Unknown,
            Some(2),
            "match arm repeats or names a foreign variant",
            "provide each exact declared variant once",
        ),
        (
            InvalidMatch::PayloadlessBinding,
            Some(0),
            "match payload binding does not match its variant",
            "bind exactly one name for a payload variant and none otherwise",
        ),
        (
            InvalidMatch::MissingPayloadBinding,
            Some(1),
            "match payload binding does not match its variant",
            "bind exactly one name for a payload variant and none otherwise",
        ),
    ];
    for (invalid, arm_index, message, guidance) in cases {
        let (text, raw) = structured_owned_fixture::invalid_mixed_variant_fixture(invalid);
        let matching = raw.files[0].functions[0]
            .body
            .expressions
            .iter()
            .find(|expression| {
                matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
            })
            .expect("match expression");
        let zryna_syntax::v4::RawExpressionKind::Match { arms, .. } = &matching.kind else {
            unreachable!("selected match expression")
        };
        let at = arm_index.map_or(matching.span, |index| arms[index].span);
        let sources = sources_for(&text);
        if matches!(invalid, InvalidMatch::Duplicate) {
            let first = verify_snapshot(raw.clone(), &sources).expect_err("duplicate arm rejected");
            assert_eq!(
                first,
                verify_snapshot(raw, &sources).expect_err("deterministic duplicate replay")
            );
            assert_eq!(first.len(), 1);
            assert_eq!(first[0].code(), "ZRYNA-Y4002");
            assert_eq!(first[0].message(), message);
            assert_eq!(first[0].guidance(), guidance);
            assert_valid_match_recovery();
            continue;
        }
        let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid match");
        let first = lower(pair_input(&syntax, &sources)).expect_err("invalid match rejected");
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources)).expect_err("deterministic invalid replay"),
            "{invalid:?}"
        );
        assert_eq!(first.len(), 1, "{invalid:?}");
        assert_eq!(first[0].code(), "ZRYNA-M3009", "{invalid:?}");
        assert_eq!(first[0].primary_span(), Some(span(&sources, at)), "{invalid:?}");
        assert_eq!(first[0].message(), message, "{invalid:?}");
        assert_eq!(first[0].guidance(), guidance, "{invalid:?}");

        assert_valid_match_recovery();
    }
}

fn assert_valid_match_recovery() {
    let (text, raw) = structured_owned_fixture::mixed_variant_fixture();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated recovery match");
    lower(pair_input(&syntax, &sources)).expect("recovery after invalid match");
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
