use super::*;
use super::generic_vec_fixture::ordinary_array_composition_fixture::copy_match_expression_fixture::{Arm, fixture};

#[test]
fn copy_match_expressions_preserve_terminal_refinement_and_dense_cross_arm_values() {
    for arm in [Arm::Add, Arm::Call, Arm::Array, Arm::Enum, Arm::Struct] {
        for reverse in [false, true] {
            let (source, raw) = fixture(arm, true, reverse);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated Copy match arms");
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
                let function = program
                    .modules()
                    .next()
                    .expect("bounded authenticated fixture")
                    .functions()
                    .next()
                    .expect("bounded authenticated fixture");
                let blocks = function.blocks().collect::<Vec<_>>();
                assert_eq!(blocks.len(), 3);
                assert_eq!(blocks[0].instructions().len(), 0);
                assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
                let values = blocks
                    .iter()
                    .flat_map(|block| block.instructions())
                    .filter_map(FaultVerifiedInstruction::result)
                    .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
                    .collect::<Vec<_>>();
                assert_eq!(
                    values,
                    (1..=u32::try_from(values.len()).expect("bounded authenticated fixture"))
                        .collect::<Vec<_>>()
                );
                for block in &blocks[1..] {
                    assert_eq!(block.parameters().len(), 0);
                    assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return);
                    let instructions = block.instructions().collect::<Vec<_>>();
                    let last = instructions.last().expect("bounded authenticated fixture");
                    assert_eq!(
                        block.terminator().value_operands().collect::<Vec<_>>(),
                        vec![last.result().expect("bounded authenticated fixture")]
                    );
                    assert_eq!(
                        last.kind(),
                        match arm {
                            Arm::Add => VerifiedInstructionKind::I32Add,
                            Arm::Call => VerifiedInstructionKind::DirectCall,
                            Arm::Array => VerifiedInstructionKind::FixedArrayConstruct,
                            Arm::Enum => VerifiedInstructionKind::EnumConstruct,
                            Arm::Struct => VerifiedInstructionKind::StructConstruct,
                            _ => unreachable!(),
                        }
                    );
                }
                let expected_plans = if matches!(arm, Arm::Call) { 4 } else { 2 };
                assert_eq!(function.cleanup_plans().len(), expected_plans);
                assert!(function.cleanup_plans().all(|plan| plan.actions().len() == 0));
            }
        }
    }
}

#[test]
fn copy_match_expressions_reject_invalid_operands_and_nonexhaustive_matches_deterministically() {
    for (arm, exhaustive, code, start, end, message, guidance) in [
        (
            Arm::WrongType,
            true,
            "ZRYNA-M3007",
            144,
            152,
            "right operand has a different exact aggregate type",
            "use a value with the exact declared type",
        ),
        (
            Arm::Missing,
            true,
            "ZRYNA-M3002",
            144,
            151,
            "name 'missing' is not declared",
            "reference one exact parameter, local, or match payload binding",
        ),
        (
            Arm::Add,
            false,
            "ZRYNA-M3009",
            108,
            161,
            "enum match has 1 arms but 'Maybe' has 2 variants",
            "provide every variant exactly once and no extra arms",
        ),
        (
            Arm::Mismatch,
            true,
            "ZRYNA-M3009",
            144,
            148,
            "enum match arms do not all have the declared result type",
            "make every arm produce one exact common type",
        ),
        (
            Arm::Nested,
            true,
            "ZRYNA-M3009",
            144,
            224,
            "nested aggregate operations in match arms are outside the M3 match oracle",
            "return a scalar literal, parameter, or payload binding from each arm",
        ),
        (
            Arm::FreshScrutinee,
            true,
            "ZRYNA-M3009",
            114,
            127,
            "enum match scrutinee must be an addressable place",
            "match a parameter or initialized local enum place",
        ),
    ] {
        let (source, raw) = fixture(arm, exhaustive, false);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated invalid Copy match");
        let first = lower(pair_input(&syntax, &sources)).expect_err("rejected match");
        let second = lower(pair_input(&syntax, &sources)).expect_err("replayed rejection");
        assert_eq!(first, second);
        let expected = zryna_diagnostics::Diagnostic::error_at(
            code,
            sources
                .verify_span(zryna_source::UntrustedSpan { file: 0, start, end })
                .expect("frozen diagnostic source span"),
            message,
            guidance,
        );
        assert_eq!(first, vec![expected], "{arm:?}");
    }
}

#[test]
fn copy_match_expressions_share_static_parameter_projections_across_refined_terminal_arms() {
    for reverse in [false, true] {
        let (source, raw) = fixture(Arm::Projection, true, reverse);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated projected arms");
        let program = lower(pair_input(&syntax, &sources)).expect("Copy projection in both arms");
        let function = program.modules().next().expect("module").functions().next().expect("get");
        let places = function.places().collect::<Vec<_>>();
        assert_eq!(places.len(), 4, "two roots, one reused index, one active payload");
        assert_eq!(
            places.iter().map(|place| place.id().index()).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(places[0].kind(), VerifiedPlaceKind::Parameter(0));
        assert_eq!(places[1].kind(), VerifiedPlaceKind::Parameter(1));
        assert_eq!(
            places[2].kind(),
            VerifiedPlaceKind::FixedArrayConstant { base: places[1].id(), index: 0 }
        );
        assert_eq!(
            places[3].kind(),
            VerifiedPlaceKind::EnumPayload { base: places[0].id(), variant: 1 }
        );
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].instructions().len(), 0);
        assert_eq!(
            blocks[0].terminator().place_operands().collect::<Vec<_>>(),
            vec![places[0].id()]
        );
        let arms = blocks[0].terminator().enum_arms().collect::<Vec<_>>();
        assert_eq!(arms.len(), 2);
        for (ordinal, arm) in arms.iter().enumerate() {
            assert_eq!(arm.variant(), u32::try_from(ordinal).expect("variant"));
            assert_eq!(arm.edge().target(), blocks[ordinal + 1].id());
        }
        for (ordinal, block) in blocks[1..].iter().enumerate() {
            let instructions = block.instructions().collect::<Vec<_>>();
            assert_eq!(
                instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
                vec![
                    VerifiedInstructionKind::CopyFromPlace,
                    if ordinal == 0 {
                        VerifiedInstructionKind::I32Literal
                    } else {
                        VerifiedInstructionKind::CopyFromPlace
                    },
                    VerifiedInstructionKind::I32Add
                ]
            );
            assert_eq!(instructions[0].place_operands().collect::<Vec<_>>(), vec![places[2].id()]);
            assert_eq!(
                instructions[1].place_operands().collect::<Vec<_>>(),
                if ordinal == 0 { Vec::new() } else { vec![places[3].id()] }
            );
            let results = instructions
                .iter()
                .map(|instruction| instruction.result().expect("Copy result"))
                .collect::<Vec<_>>();
            let first = u32::try_from(2 + ordinal * 3).expect("dense first value");
            assert_eq!(
                results.iter().map(|value| value.index()).collect::<Vec<_>>(),
                vec![first, first + 1, first + 2]
            );
            assert_eq!(instructions[2].value_operands().collect::<Vec<_>>(), results[..2]);
            assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return);
            assert_eq!(block.terminator().value_operands().collect::<Vec<_>>(), vec![results[2]]);
        }
    }
}
