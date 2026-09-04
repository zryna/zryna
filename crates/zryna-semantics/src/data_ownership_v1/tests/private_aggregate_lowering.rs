use super::*;

mod constructor_order;

#[test]
fn owned_struct_initializers_preserve_declared_evaluation_order_and_source_spans() {
    for (source, response, expected) in [
        (
            OWNED_TRIO_SOURCE,
            OWNED_TRIO_RESPONSE,
            vec![(b'a', 129, 132), (b'b', 121, 124), (b'c', 113, 116)],
        ),
        (NESTED_OWNED_SOURCE, NESTED_OWNED_RESPONSE, vec![(b'a', 194, 197), (b'b', 168, 171)]),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("original source-order syntax remains authenticated");
        let program = lower(pair_input(&syntax, &sources)).expect("owned constructor verifies");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        let actual = instructions
            .iter()
            .filter_map(|instruction| {
                instruction.string_utf8_bytes().map(|bytes| {
                    assert_eq!(bytes.len(), 1);
                    (bytes[0], instruction.span().start(), instruction.span().end())
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "prepare declarations, not constructor spelling order");
        if source == NESTED_OWNED_SOURCE {
            assert_eq!(
                instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
                vec![
                    VerifiedInstructionKind::StringFromUtf8,
                    VerifiedInstructionKind::StructConstruct,
                    VerifiedInstructionKind::StringFromUtf8,
                    VerifiedInstructionKind::StructConstruct,
                ]
            );
        }
    }
}

#[test]
fn private_owned_fixed_array_prepares_indices_and_moves_whole_result() {
    let sources = sources_for(OWNED_ARRAY_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ARRAY_RESPONSE), &sources)
        .expect("source-faithful owned array v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned array must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::FixedArrayConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    assert_eq!(instructions[0].derived_drop_actions().count(), 0);
    assert_eq!(instructions[1].derived_drop_actions().count(), 1);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(
        instructions[2]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![0, 1],
    );
}
#[test]
fn nested_owned_structs_consume_inner_owner_once_and_preserve_failure_cleanup() {
    let sources = sources_for(NESTED_OWNED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(NESTED_OWNED_RESPONSE), &sources)
        .expect("source-faithful nested owned aggregate v4");
    let program = lower(pair_input(&syntax, &sources)).expect("nested owned aggregate must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
        ]
    );
    assert_eq!(instructions[0].derived_drop_actions().count(), 0);
    assert_eq!(instructions[1].cleanup(), None);
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        instructions[3]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![1, 2],
        "inner commits before tail preparation in outer declaration order",
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}
#[test]
fn reversed_owned_fields_have_reverse_prepare_cleanup_and_canonical_commit_operands() {
    let sources = sources_for(OWNED_TRIO_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_TRIO_RESPONSE), &sources)
        .expect("source-faithful reversed owned fields v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Trio must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    for (index, instruction) in instructions[..3].iter().enumerate() {
        assert_eq!(
            instruction
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            (0..u32::try_from(index).expect("three initializers")).rev().collect::<Vec<_>>(),
            "each fallible initializer cleans only the actual completed prefix",
        );
    }
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1, 0],
        "third fallible leaf drops the prepared prefix in reverse completion order",
    );
    assert_eq!(
        instructions[3]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2],
        "a/b/c preparation and commit both follow declaration order",
    );
    assert_eq!(instructions[3].cleanup(), None);
}
#[test]
fn owned_struct_with_fixed_array_child_commits_each_nested_owner_once() {
    let sources = sources_for(OWNED_CROSS_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_CROSS_RESPONSE), &sources)
        .expect("source-faithful Struct/FixedArray v4");
    let program = lower(pair_input(&syntax, &sources)).expect("cross aggregate must verify");
    let instructions = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        instructions,
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::FixedArrayConstruct,
            VerifiedInstructionKind::StructConstruct,
        ]
    );
}
#[test]
fn private_owned_enum_payloadless_and_copy_payloads_commit_infallibly() {
    for (source, response, expected) in [
        (
            OWNED_ENUM_NONE_SOURCE,
            OWNED_ENUM_NONE_RESPONSE,
            vec![VerifiedInstructionKind::EnumConstruct],
        ),
        (
            OWNED_ENUM_COPY_SOURCE,
            OWNED_ENUM_COPY_RESPONSE,
            vec![VerifiedInstructionKind::I32Literal, VerifiedInstructionKind::EnumConstruct],
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful owned enum v4");
        let program = lower(pair_input(&syntax, &sources)).expect("owned enum must verify");
        let block = program
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .next()
            .expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
            expected,
        );
        let construct = instructions.last().expect("enum construction");
        assert_eq!(construct.cleanup(), None);
        assert_eq!(construct.variant(), Some(u32::from(instructions.len() == 2)));
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    }
}
#[test]
fn private_owned_enum_string_move_and_survivor_cleanup_are_exact() {
    let sources = sources_for(OWNED_ENUM_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_STRING_RESPONSE), &sources)
        .expect("source-faithful String enum v4");
    let program = lower(pair_input(&syntax, &sources)).expect("String enum must verify");
    let block = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::EnumConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ],
    );
    assert_eq!(
        instructions[2]
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1],
        "payload preparation failure retains the preceding survivor",
    );
    assert_eq!(instructions[3].cleanup(), None);
    assert_eq!(instructions[3].variant(), Some(1));
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        vec![1],
        "return transfer excludes only the returned enum and drops survivors in reverse order",
    );
}
#[test]
fn private_owned_enum_accepts_supported_nested_aggregate_payload() {
    let sources = sources_for(OWNED_ENUM_NESTED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_NESTED_RESPONSE), &sources)
        .expect("source-faithful nested enum payload v4");
    let program = lower(pair_input(&syntax, &sources)).expect("nested enum payload must verify");
    let instructions = program
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::EnumConstruct,
        ],
    );
    assert_eq!(instructions[1].cleanup(), None);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(instructions[2].variant(), Some(1));
}
#[test]
fn private_owned_enum_use_after_move_and_exclusions_fail_closed() {
    let sources = sources_for(OWNED_ENUM_MOVED_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_MOVED_RESPONSE), &sources)
        .expect("source-faithful moved enum v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("second enum move");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((155, 156)),
    );

    let sources = sources_for(OWNED_ENUM_VEC_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_ENUM_VEC_RESPONSE), &sources)
        .expect("source-faithful excluded Vec payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Vec enum payload excluded");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
}
#[test]
fn private_owned_enum_wrong_payload_shape_uses_enum_diagnostic() {
    let source = OWNED_ENUM_NONE_SOURCE.replace("Maybe.none()", "Maybe.some()");
    let mut raw = response_snapshot(OWNED_ENUM_NONE_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut raw.files[0].functions[0].body.expressions[0].kind
    else {
        panic!("enum construction")
    };
    variant.text = "some".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful missing payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("missing enum payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");

    let source = OWNED_ENUM_COPY_SOURCE.replace("Maybe.some(7)", "Maybe.none(7)");
    let mut raw = response_snapshot(OWNED_ENUM_COPY_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::EnumConstruction { variant, .. } =
        &mut raw.files[0].functions[0].body.expressions[1].kind
    else {
        panic!("enum construction")
    };
    variant.text = "none".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful extra payload v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("extra enum payload");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3005");
}
#[test]
fn private_owned_aggregate_requires_exactly_one_final_return() {
    for (source, response, expected_span) in [
        (OWNED_ENUM_DUP_RETURN_SOURCE, OWNED_ENUM_DUP_RETURN_RESPONSE, (115, 135)),
        (OWNED_ENUM_LOCAL_AFTER_RETURN_SOURCE, OWNED_ENUM_LOCAL_AFTER_RETURN_RESPONSE, (115, 145)),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful invalid return structure v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("return structure");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
        assert_eq!(
            diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
            Some(expected_span),
        );
    }
}
#[test]
fn owned_aggregate_unavailable_and_excluded_shape_diagnostics_are_stable() {
    let mut unavailable_source = OWNED_PAIR_SOURCE.to_owned();
    unavailable_source.replace_range(167..168, "P");
    let mut unavailable = response_snapshot(OWNED_PAIR_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &mut unavailable.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("return reference");
    };
    name.text = "P".to_owned();
    let sources = sources_for(&unavailable_source);
    let syntax = verify_snapshot(unavailable, &sources).expect("wrong-case source-faithful v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unavailable aggregate");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((167, 168)),
    );

    let mut duplicate_source = OWNED_TRIO_SOURCE.to_owned();
    duplicate_source.replace_range(118..119, "z");
    let mut duplicate = response_snapshot(OWNED_TRIO_RESPONSE);
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut duplicate.files[0].functions[0].body.expressions[3].kind
    else {
        panic!("Trio constructor");
    };
    let zryna_syntax::v4::RawFieldInitializerKind::Explicit { name, .. } = &mut fields[1].kind
    else {
        panic!("explicit field");
    };
    name.text = "z".to_owned();
    let sources = sources_for(&duplicate_source);
    let syntax = verify_snapshot(duplicate, &sources).expect("unknown field source-faithful v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("excluded unknown field");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((118, 124)),
    );
}
#[test]
fn reversed_struct_fields_evaluate_and_construct_in_declaration_order() {
    let mut source = PAIR_SCORE_SOURCE.to_owned();
    source.replace_range(137..148, "right, left");
    let sources = sources_for(&source);
    let mut raw = decode_snapshot(PAIR_SCORE_JSON).expect("Pair score JSON");
    let expressions = &mut raw.files[0].functions[0].body.expressions;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[0].kind else {
        panic!("first reference")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    expressions[0].span.end = 142;
    let zryna_syntax::v4::RawExpressionKind::Reference { name } = &mut expressions[1].kind else {
        panic!("second reference")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    expressions[1].span.start = 144;
    let zryna_syntax::v4::RawExpressionKind::StructConstruction { fields, .. } =
        &mut expressions[2].kind
    else {
        panic!("constructor")
    };
    fields[0].span.end = 142;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[0].kind
    else {
        panic!("first field")
    };
    name.text = "right".to_owned();
    name.span.end = 142;
    fields[1].span.start = 144;
    let zryna_syntax::v4::RawFieldInitializerKind::Shorthand { name, .. } = &mut fields[1].kind
    else {
        panic!("second field")
    };
    name.text = "left".to_owned();
    name.span.start = 144;
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful reversed fields");
    let program = lower(pair_input(&syntax, &sources)).expect("reversed fields must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let copies = instructions
        .iter()
        .copied()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
        .take(2)
        .collect::<Vec<_>>();
    let first_place = copies[0].place_operands().next().expect("first source operand");
    let second_place = copies[1].place_operands().next().expect("second source operand");
    assert!(matches!(
        places[usize::try_from(first_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(0)
    ));
    assert!(matches!(
        places[usize::try_from(second_place.index()).expect("place")].kind(),
        VerifiedPlaceKind::Parameter(1)
    ));
    let construct = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StructConstruct)
        .expect("construct");
    let operands = construct
        .value_operands()
        .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
        .collect::<Vec<_>>();
    assert_eq!(
        operands,
        vec![
            copies[0].result().expect("left result").index(),
            copies[1].result().expect("right result").index()
        ]
    );
}
