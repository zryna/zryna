use super::*;

#[test]
fn static_subobject_move_rejects_the_wrong_exact_contextual_type() {
    let mut source = PROJECTED_INNER_MOVE_SOURCE.to_owned();
    let type_start =
        source.find("const moved: Inner").expect("moved declaration") + "const moved: ".len();
    source.replace_range(type_start..type_start + "Inner".len(), "Outer");
    let mut raw = response_snapshot(PROJECTED_INNER_MOVE_RESPONSE);
    let RawTypeSyntaxKind::Named { name } = &mut raw.files[0].type_syntax[5].kind else {
        panic!("moved local named type")
    };
    name.text = "Outer".to_owned();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful wrong projected type");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("wrong projected type");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
}

#[test]
fn static_subobject_move_rejects_child_reuse_after_parent_transfer() {
    let (source, raw) = projected_inner_child_after_parent_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful child-after-parent use");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("child after aggregate parent move");
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
}

#[test]
fn complete_static_subobject_moves_directly_into_the_final_return() {
    for (source, response, label) in [
        (
            PROJECTED_INNER_DIRECT_RETURN_SOURCE,
            PROJECTED_INNER_DIRECT_RETURN_RESPONSE,
            "StructField",
        ),
        (
            FIXED_ARRAY_SUBOBJECT_RETURN_SOURCE.trim_end(),
            FIXED_ARRAY_SUBOBJECT_RETURN_RESPONSE,
            "FixedArrayConstant",
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(response_snapshot(response), &sources)
            .expect("source-faithful direct projected return");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let source_root = function
            .places()
            .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
            .expect("source root")
            .id();
        let block = function.blocks().next().expect("block");
        let projection_move = block
            .instructions()
            .find(|instruction| {
                instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                    && instruction.place_operands().next().is_some_and(|place| {
                        function.places().find(|candidate| candidate.id() == place).is_some_and(
                            |source| match (label, source.kind()) {
                                (
                                    "StructField",
                                    VerifiedPlaceKind::StructField { base, ordinal },
                                ) => base == source_root && ordinal == 0,
                                (
                                    "FixedArrayConstant",
                                    VerifiedPlaceKind::FixedArrayConstant { base, index },
                                ) => base == source_root && index == 0,
                                _ => false,
                            },
                        )
                    })
            })
            .expect("projected aggregate move");
        let returned = block.terminator().value_operands().next().expect("returned value");
        assert_eq!(projection_move.result(), Some(returned));
        let returned_owner = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == returned)
            })
            .expect("returned temporary")
            .id();
        let source_cleanup = block
            .terminator()
            .derived_drop_actions()
            .find(|action| action.root() == source_root)
            .expect("masked source cleanup");
        let moved = source_cleanup.moved_projections().collect::<Vec<_>>();
        assert_eq!(moved.len(), 2, "{label}");
        assert!(
            block.terminator().derived_drop_actions().all(|action| action.root() != returned_owner)
        );
    }
}

#[test]
fn direct_static_subobject_return_rejects_parameters_before_lowering_the_move() {
    let (source, raw) = projected_aggregate_direct_return_with_parameter_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful parameterized return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("parameterized return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "return o.inner;", 0)))
    );
}

#[test]
fn direct_static_subobject_return_rejects_a_nonfinal_first_site_before_lowering() {
    const INSERTION: u32 = 231;
    const SUFFIX: &str = " return o.inner;";
    let mut source = PROJECTED_INNER_DIRECT_RETURN_SOURCE.to_owned();
    source.insert_str(usize::try_from(INSERTION).expect("insertion"), SUFFIX);
    let mut raw = shift_snapshot(
        response_snapshot(PROJECTED_INNER_DIRECT_RETURN_RESPONSE),
        INSERTION,
        u32::try_from(SUFFIX.len()).expect("suffix length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let reference = u32::try_from(body.expressions.len()).expect("reference id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 240 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "o".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 240 },
            },
        },
    });
    let projection = u32::try_from(body.expressions.len()).expect("projection id");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 239, end: 246 },
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base: reference,
            dot_span: zryna_source::UntrustedSpan { file: 0, start: 240, end: 241 },
            field: RawIdentifierSyntax {
                text: "inner".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: 241, end: 246 },
            },
        },
    });
    let statement = u32::try_from(body.statements.len()).expect("statement id");
    body.statements.push(RawStatementSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: 232, end: 247 },
        kind: RawStatementKind::Return {
            keyword_span: zryna_source::UntrustedSpan { file: 0, start: 232, end: 238 },
            value: projection,
            semicolon_span: zryna_source::UntrustedSpan { file: 0, start: 246, end: 247 },
        },
    });
    body.blocks[0].statements.push(statement);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nonfinal return");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("nonfinal projected return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3010");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "return o.inner;", 1)))
    );
}

#[test]
fn direct_static_subobject_return_resource_preflight_is_exact_and_checked() {
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let actions = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!projected_subobject_return_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        plans - 1,
        actions - 2,
        2,
        0,
        2,
    ));
    assert!(!projected_subobject_return_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        plans - 1,
        actions - 2,
        2,
        1,
        1,
    ));
    for violation in [
        projected_subobject_return_budget_violation(
            values,
            places - 3,
            transitions - 1,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 2,
            transitions - 1,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions,
            0,
            plans - 1,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions - 1,
            0,
            plans,
            actions - 2,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(
            values - 1,
            places - 3,
            transitions - 1,
            0,
            plans - 1,
            actions - 1,
            2,
            0,
            2,
        ),
        projected_subobject_return_budget_violation(0, 0, 0, 0, 0, 0, 0, usize::MAX, usize::MAX),
    ] {
        assert!(violation);
    }
}

#[test]
fn static_subobject_move_resource_preflight_is_exact_and_checked() {
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!projected_subobject_move_budget_violation(
        values - 1,
        places - 3,
        transitions - 1,
        0,
        2,
    ));
    assert!(projected_subobject_move_budget_violation(values, places - 3, transitions - 1, 0, 2,));
    assert!(projected_subobject_move_budget_violation(
        values - 1,
        places - 2,
        transitions - 1,
        0,
        2,
    ));
    assert!(projected_subobject_move_budget_violation(values - 1, places - 3, transitions, 0, 2,));
    assert!(projected_subobject_move_budget_violation(0, 0, 0, 0, usize::MAX,));
}
