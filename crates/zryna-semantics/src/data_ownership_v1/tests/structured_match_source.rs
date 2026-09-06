use super::*;

#[path = "structured_match_complete_source.rs"]
mod complete;

#[test]
fn structured_match_vec_growth_cleanup_retains_both_completed_operands() {
    for cloned in [false, true] {
        let (text, raw) = structured_owned_fixture::vec_match_fixture(cloned, true);
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated Vec match operand");
        let program = lower(pair_input(&syntax, &sources)).expect("Vec constructor continuation");
        let function = program
            .verified_ir()
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function");
        let blocks = function.blocks().collect::<Vec<_>>();
        let growth = blocks[3].instructions().next().expect("Vec growth after join");
        assert_eq!(growth.kind(), VerifiedInstructionKind::VecConstruct);
        let operands = growth.value_operands().collect::<Vec<_>>();
        assert_eq!(operands.len(), 2);
        let owners = operands.iter().map(|&value| function.places().find(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(actual) if actual == value)).expect("complete child owner").id()).collect::<Vec<_>>();
        assert_eq!(
            growth.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
            vec![owners[1], owners[0]]
        );
    }
}

#[test]
fn structured_match_call_missing_target_rejects_before_argument_effects_and_replays() {
    let (mut text, mut raw) = structured_owned_fixture::call_match_fixture(true, true);
    let expression = raw.files[0].functions[0]
        .body
        .expressions
        .iter_mut()
        .find(|expression| {
            matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Call { .. })
        })
        .expect("internal call");
    let zryna_syntax::v4::RawExpressionKind::Call { callee, .. } = &mut expression.kind else {
        unreachable!("selected call")
    };
    let at = callee.span;
    text.replace_range(at.start as usize..at.end as usize, "unknown");
    callee.text = "unknown".into();
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated unresolved call");
    let first = lower(pair_input(&syntax, &sources)).expect_err("missing target rejected");
    assert_eq!(first, lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection"));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-M3002");
    assert_eq!(first[0].primary_span(), Some(span(&sources, at)));
    assert_eq!(first[0].message(), "function 'unknown' is not declared in this module");
    assert_eq!(first[0].guidance(), "call one exact private same-module function");
}

#[test]
fn structured_match_call_retains_arguments_until_complete_then_transfers_before_trap() {
    for cloned in [false, true] {
        for local in [false, true] {
            let (text, raw) = structured_owned_fixture::call_match_fixture(cloned, local);
            let sources = sources_for(&text);
            let syntax = verify_snapshot(raw, &sources)
                .expect("authenticated private call with argument match");
            let program = lower(pair_input(&syntax, &sources)).expect("structured call verifies");
            let function = program
                .verified_ir()
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("caller");
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4);
            let first = blocks[0].instructions().next().expect("first String argument");
            assert_eq!(first.kind(), VerifiedInstructionKind::StringFromUtf8);
            let call = blocks[3].instructions().next().expect("call after complete match");
            assert_eq!(call.kind(), VerifiedInstructionKind::DirectCall);
            assert_eq!(
                call.value_operands().collect::<Vec<_>>(),
                vec![
                    first.result().expect("first argument"),
                    blocks[3].parameters().next().expect("joined second argument").id()
                ]
            );
            assert_eq!(call.derived_drop_actions().len(), 0);
            if cloned {
                for arm in &blocks[1..3] {
                    assert_eq!(
                        arm.instructions()
                            .next()
                            .expect("fallible second argument")
                            .derived_drop_actions()
                            .len(),
                        2
                    );
                }
            }
            let replay = lower(pair_input(&syntax, &sources)).expect("deterministic call replay");
            assert_eq!(
                format!("{:?}", program.verified_ir()),
                format!("{:?}", replay.verified_ir())
            );
        }
    }
}

#[test]
fn structured_match_constructor_retains_earlier_operand_across_continuation() {
    for cloned in [false, true] {
        for local in [false, true] {
            let (text, raw) = structured_owned_fixture::nested_match_fixture(cloned, local);
            let sources = sources_for(&text);
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated nested constructor match");
            let program =
                lower(pair_input(&syntax, &sources)).expect("constructor resumes after match");
            let function = program
                .verified_ir()
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("function");
            let blocks = function.blocks().collect::<Vec<_>>();
            assert_eq!(blocks.len(), 4);
            let first = blocks[0].instructions().next().expect("earlier String operand");
            assert_eq!(first.kind(), VerifiedInstructionKind::StringFromUtf8);
            let commit = blocks[3].instructions().next().expect("resumed constructor");
            assert_eq!(commit.kind(), VerifiedInstructionKind::FixedArrayConstruct);
            assert_eq!(commit.value_operands().next(), first.result());
            assert_eq!(commit.value_operands().len(), 2);
            if cloned {
                let owner = function.places().find(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if Some(value) == first.result())).expect("earlier genuine String owner").id();
                for arm in &blocks[1..3] {
                    let cleanup = arm
                        .instructions()
                        .next()
                        .expect("fallible payload clone")
                        .derived_drop_actions()
                        .collect::<Vec<_>>();
                    assert_eq!(cleanup.len(), 2);
                    assert_eq!(cleanup[0].root(), owner);
                    assert_ne!(cleanup[1].root(), owner);
                }
            }
            assert_eq!(
                commit.value_operands().nth(1),
                Some(blocks[3].parameters().next().expect("match result").id())
            );
        }
    }
}

#[test]
fn structured_match_owned_payloads_join_one_result_and_continue() {
    use structured_owned_fixture::Payload;
    for payload in [
        Payload::String,
        Payload::Array(0),
        Payload::Array(2),
        Payload::Vec,
        Payload::Nested,
        Payload::Struct,
        Payload::Enum,
    ] {
        for cloned in [false, true] {
            for local in [false, true] {
                let (text, raw) = structured_owned_fixture::match_fixture(payload, cloned, local);
                let sources = sources_for(&text);
                let syntax =
                    verify_snapshot(raw, &sources).expect("authenticated two-arm owned match");
                let program = lower(pair_input(&syntax, &sources))
                    .expect("owned match continuation verifies");
                let function = program
                    .verified_ir()
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function");
                let blocks = function.blocks().collect::<Vec<_>>();
                assert_eq!(blocks.len(), 4);
                assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
                assert_eq!(blocks[3].parameters().len(), 1);
                for (ordinal, arm) in blocks[1..3].iter().enumerate() {
                    assert_eq!(
                        arm.terminator()
                            .edges()
                            .next()
                            .expect("continuation edge")
                            .arguments()
                            .len(),
                        1
                    );
                    let expected = if !cloned {
                        VerifiedInstructionKind::GenericMoveFromPlace
                    } else if matches!(payload, Payload::String) {
                        VerifiedInstructionKind::StringClone
                    } else {
                        VerifiedInstructionKind::GenericClonePlace
                    };
                    assert_eq!(
                        arm.instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
                        vec![expected, VerifiedInstructionKind::DropPlace]
                    );
                    let dropped = arm
                        .instructions()
                        .last()
                        .expect("enum cleanup")
                        .derived_drop_actions()
                        .collect::<Vec<_>>();
                    assert_eq!(dropped.len(), 1);
                    assert_eq!(
                        dropped[0].active_variant(),
                        Some(u32::try_from(ordinal).expect("two variants"))
                    );
                    assert_eq!(dropped[0].moved_projections().len(), usize::from(!cloned));
                }
            }
        }
    }
}

#[test]
fn structured_match_bad_arm_values_reject_and_replay_exact_diagnostics() {
    use zryna_syntax::v4::RawExpressionKind;
    for wrong_type in [false, true] {
        let (mut text, mut raw) = structured_owned_fixture::match_fixture(
            structured_owned_fixture::Payload::String,
            false,
            true,
        );
        let expression = raw.files[0].functions[0].body.expressions.iter_mut().find(|expression| matches!(&expression.kind, RawExpressionKind::Reference { name } if name.text == "first")).expect("first payload use");
        let untrusted = expression.span;
        let replacement = if wrong_type { "false" } else { "third" };
        text.replace_range(untrusted.start as usize..untrusted.end as usize, replacement);
        expression.kind = if wrong_type {
            RawExpressionKind::BoolLiteral { value: false }
        } else {
            RawExpressionKind::Reference {
                name: RawIdentifierSyntax { text: replacement.into(), span: untrusted },
            }
        };
        let sources = sources_for(&text);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated bad arm expression");
        let first = lower(pair_input(&syntax, &sources)).expect_err("bad arm rejects");
        assert_eq!(
            first,
            lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection")
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), if wrong_type { "ZRYNA-M3016" } else { "ZRYNA-M3002" });
        assert_eq!(first[0].primary_span(), Some(span(&sources, untrusted)));
    }
}

#[test]
fn structured_match_copy_payload_continuation_retains_surrounding_owned_parameter() {
    let (text, raw) = structured_owned_fixture::match_fixture(
        structured_owned_fixture::Payload::I32,
        false,
        true,
    );
    let sources = sources_for(&text);
    let syntax = verify_snapshot(raw, &sources)
        .expect("authenticated Copy match with owned surrounding state");
    let program = lower(pair_input(&syntax, &sources)).expect("Copy payload continuation");
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        blocks[0].instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
        vec![VerifiedInstructionKind::CopyFromPlace, VerifiedInstructionKind::InitializePlace]
    );
    for (ordinal, arm) in blocks[1..3].iter().enumerate() {
        assert_eq!(
            arm.instructions().map(FaultVerifiedInstruction::kind).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::CopyFromPlace,
                VerifiedInstructionKind::BeginBorrow,
                VerifiedInstructionKind::BorrowWrite,
                VerifiedInstructionKind::EndBorrow
            ]
        );
        let instructions = arm.instructions().collect::<Vec<_>>();
        let original = blocks[0]
            .instructions()
            .next()
            .expect("once-evaluated Copy scrutinee")
            .result()
            .expect("Copy result");
        assert_eq!(instructions[2].value_operands().collect::<Vec<_>>(), vec![original]);
        let temporary = blocks[0]
            .instructions()
            .nth(1)
            .expect("private initialization")
            .place_operands()
            .next()
            .expect("temporary");
        assert_eq!(instructions[1].place_operands().collect::<Vec<_>>(), vec![temporary]);
        let borrow = instructions[1].borrow().expect("private exclusive authority");
        assert_eq!(borrow.index(), u32::try_from(ordinal).expect("two arm authorities"));
        assert_eq!(instructions[2].borrow(), Some(borrow));
        assert_eq!(instructions[3].borrow(), Some(borrow));
        assert_eq!(
            arm.terminator().edges().next().expect("join edge").arguments().collect::<Vec<_>>(),
            vec![instructions[0].result().expect("payload Copy value")]
        );
        for instruction in instructions {
            assert_eq!(instruction.derived_drop_actions().len(), 0);
        }
    }
    assert_eq!(blocks[3].terminator().derived_drop_actions().len(), 1);
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic Copy match replay");
    assert_eq!(format!("{:?}", program.verified_ir()), format!("{:?}", replay.verified_ir()));
}
