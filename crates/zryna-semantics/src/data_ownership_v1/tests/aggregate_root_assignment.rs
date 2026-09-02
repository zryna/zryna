use super::*;

fn owned_fixed_array_clone_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "a = clone(a); ";
    let mut source = OWNED_ARRAY_SOURCE.to_owned();
    source.replace_range(41..46, "let  ");
    let insertion = source.find("return a;").expect("array return insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("array insertion");
    let mut raw = shift_snapshot(
        response_snapshot(OWNED_ARRAY_RESPONSE),
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("array assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned array local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let target = u32::try_from(body.expressions.len()).expect("array target");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 1 },
            },
        },
    });
    let source_value = u32::try_from(body.expressions.len()).expect("array clone source");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion + 10, end: insertion + 11 },
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax {
                text: "a".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 10,
                    end: insertion + 11,
                },
            },
        },
    });
    let cloned = u32::try_from(body.expressions.len()).expect("array clone");
    body.expressions.push(RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: insertion + 4, end: insertion + 12 },
        kind: zryna_syntax::v4::RawExpressionKind::Clone {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 4,
                end: insertion + 9,
            },
            open_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 9,
                end: insertion + 10,
            },
            value: source_value,
            close_paren_span: zryna_source::UntrustedSpan {
                file: 0,
                start: insertion + 11,
                end: insertion + 12,
            },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: zryna_source::UntrustedSpan { file: 0, start: insertion, end: insertion + 13 },
            kind: RawStatementKind::Assignment {
                target,
                equals_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 2,
                    end: insertion + 3,
                },
                value: cloned,
                semicolon_span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: insertion + 12,
                    end: insertion + 13,
                },
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[test]
fn string_bearing_struct_assignment_prepares_before_replacing_the_exact_root() {
    let (source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::Fresh, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful aggregate assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    let replace = instructions[replace_index];
    let target = replace.place_operands().next().expect("replacement target");
    let prepared_string = instructions[..replace_index]
        .iter()
        .rev()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("prepared String leaf");
    assert!(
        prepared_string.derived_drop_actions().any(|action| action.root() == target),
        "fallible RHS preparation retains the old aggregate root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    assert_eq!(replace_index + 2, instructions.len(), "commit precedes only the final return move");
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn partial_assignment_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_assignment_place_delta(0, 0, 0), Some(1));
    assert_eq!(partial_assignment_place_delta(2, 0, 0), Some(7));
    assert_eq!(partial_assignment_place_delta(2, 1, 0), Some(6));
    assert_eq!(partial_assignment_place_delta(2, 1, 1), Some(5));
    assert_eq!(partial_assignment_place_delta(2, 2, 2), Some(3));
    assert_eq!(partial_assignment_place_delta(1, 2, 0), None);
    assert_eq!(partial_assignment_place_delta(1, 0, 2), None);
    assert_eq!(partial_assignment_place_delta(usize::MAX, 0, 0), None);
    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 2, 0, 2, 1, 1,),
        Ok(5),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values, places - 5, transitions - 2, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 4, transitions - 2, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 1, 0, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_assignment_budget_preflight(values - 1, places - 5, transitions - 2, 1, 2, 1, 1,),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_assignment_budget_preflight(0, 0, 0, 0, usize::MAX, 0, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
}
#[test]
fn aggregate_assignment_rejects_direct_self_move_and_immutable_target() {
    for (rhs, mutable, reference_ordinal, label) in [
        (OwnedPairAssignmentRhs::SelfMove, true, 2, "direct self move"),
        (OwnedPairAssignmentRhs::Fresh, false, 1, "immutable target"),
    ] {
        let (source, raw) = owned_pair_assignment_snapshot(rhs, mutable);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful rejected assignment");
        let first = lower(pair_input(&syntax, &sources)).expect_err(label);
        let second = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(first.len(), 1, "{label}");
        assert_eq!(first[0].code(), "ZRYNA-M3014", "{label}");
        assert_eq!(
            first[0].primary_span(),
            Some(span(&sources, nth_untrusted_span(&source, "p", reference_ordinal))),
            "{label}",
        );
        assert_eq!(first[0].message(), second[0].message(), "{label}");
        assert_eq!(first[0].primary_span(), second[0].primary_span(), "{label}");
    }
}
#[test]
fn aggregate_assignment_may_copy_project_from_its_preserved_destination() {
    let (source, raw) =
        owned_pair_projection_assignment_snapshot(OwnedPairProjectionAssignmentRhs::CopyField);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful Copy projection aggregate assignment");
    let program = lower(pair_input(&syntax, &sources))
        .expect("Copy projection must not consume the preserved assignment destination");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let copy_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::CopyFromPlace)
        .expect("CopyFromPlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(copy_index < replace_index);
    let projected = instructions[copy_index].place_operands().next().expect("Copy projection");
    assert!(matches!(
        function.places().find(|place| place.id() == projected).expect("projected place").kind(),
        VerifiedPlaceKind::StructField { ordinal: 1, .. }
    ));
}
#[test]
fn aggregate_assignment_rejects_owned_projection_consumption_from_destination() {
    let (source, raw) =
        owned_pair_projection_assignment_snapshot(OwnedPairProjectionAssignmentRhs::MoveField);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful consuming projection aggregate assignment");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("destination projection consumption");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    let projection = nth_untrusted_span(&source, "p.first", 0);
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: projection.file,
                start: projection.start,
                end: projection.start + 1,
            },
        )),
    );
}
#[test]
fn fixed_array_assignment_reports_invalid_projection_before_consumption() {
    let (source, raw) = fixed_array_oob_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources)
        .expect("source-faithful out-of-bounds projection assignment");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("out-of-bounds assignment projection");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3006");
    let projection = nth_untrusted_span(&source, "a[2]", 0);
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: projection.file,
                start: projection.start + 2,
                end: projection.start + 3,
            },
        )),
    );
}
#[test]
fn root_enum_assignment_replaces_with_authenticated_old_variant_drop() {
    let (source, raw) = owned_enum_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful enum assignment v4");
    let program = lower(pair_input(&syntax, &sources)).expect("root enum assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let replace = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    let target = replace.place_operands().next().expect("enum target");
    let actions = replace.derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].root(), target);
    assert_eq!(actions[0].active_variant(), Some(1));
    assert_eq!(
        actions[0]
            .active_variants()
            .find(|variant| variant.place() == target)
            .map(VerifiedActiveVariant::variant),
        Some(1),
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 1);
}
#[test]
fn aggregate_assignment_transition_budget_is_exact_plus_one_and_overflow_checked() {
    let maximum = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert!(!aggregate_transition_budget_violation(maximum, 0, 0));
    assert!(!aggregate_transition_budget_violation(maximum - 2, 1, 1));
    assert!(aggregate_transition_budget_violation(maximum - 2, 1, 2));
    assert!(aggregate_transition_budget_violation(0, usize::MAX, 1));
    assert!(aggregate_transition_budget_violation(usize::MAX, 0, 1));
}

#[test]
fn aggregate_clone_target_assignment_retains_source_until_replace_commit() {
    let (source, raw) = owned_pair_assignment_snapshot(OwnedPairAssignmentRhs::CloneTarget, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful clone-target assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("ClonePlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(clone_index < replace_index, "clone preparation must precede commit");
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source_owner = clone.place_operands().next().expect("clone source");
    assert_eq!(replace.place_operands().next(), Some(source_owner));
    assert!(clone.derived_drop_actions().any(|action| action.root() == source_owner));
    assert!(
        clone
            .aggregate_clone_element_failure_drop_actions()
            .any(|action| action.root() == source_owner),
        "recursive clone failure retains the assignment target",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source_owner],
        "the old source is dropped only by the replacement commit",
    );
}

#[test]
fn string_fixed_array_clone_assignment_replaces_one_mutable_whole_root() {
    let (source, raw) = owned_fixed_array_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful FixedArray assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("owned FixedArray assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("ClonePlace");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("ReplacePlace");
    assert!(clone_index < replace_index);
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let target = clone.place_operands().next().expect("array source root");
    assert_eq!(replace.place_operands().next(), Some(target));
    assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(2));
    assert!(clone.derived_drop_actions().any(|action| action.root() == target));
    assert!(
        clone.aggregate_clone_element_failure_drop_actions().any(|action| action.root() == target),
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}
