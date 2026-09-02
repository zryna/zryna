use super::*;

fn owned_pair_moved_projection_assignment_snapshot() -> (String, RawProjectSyntaxSnapshot) {
    const ASSIGNMENT: &str = "p.first = \"b\"; ";
    let (mut source, mut raw) = owned_pair_partial_then_root_snapshot();
    let mutable = source.find("const p").expect("owned Pair local");
    source.replace_range(mutable..mutable + 5, "let  ");
    let insertion = source.find("return p;").expect("moved projection assignment insertion");
    source.insert_str(insertion, ASSIGNMENT);
    let insertion = u32::try_from(insertion).expect("moved projection assignment offset");
    raw = shift_snapshot(
        raw,
        insertion,
        u32::try_from(ASSIGNMENT.len()).expect("moved projection assignment length"),
    );
    let body = &mut raw.files[0].functions[0].body;
    let RawStatementKind::LocalDeclaration { keyword_span, mutable, .. } =
        &mut body.statements[0].kind
    else {
        panic!("owned Pair local")
    };
    keyword_span.end = keyword_span.start + 3;
    *mutable = true;
    let s = |start, end| zryna_source::UntrustedSpan { file: 0, start, end };
    let base = u32::try_from(body.expressions.len()).expect("moved target base");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 1),
        kind: zryna_syntax::v4::RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: "p".to_owned(), span: s(insertion, insertion + 1) },
        },
    });
    let target = u32::try_from(body.expressions.len()).expect("moved target projection");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion, insertion + 7),
        kind: zryna_syntax::v4::RawExpressionKind::FieldAccess {
            base,
            dot_span: s(insertion + 1, insertion + 2),
            field: RawIdentifierSyntax {
                text: "first".to_owned(),
                span: s(insertion + 2, insertion + 7),
            },
        },
    });
    let value = u32::try_from(body.expressions.len()).expect("moved target replacement");
    body.expressions.push(RawExpressionSyntax {
        span: s(insertion + 10, insertion + 13),
        kind: zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling: "\"b\"".to_owned() },
    });
    body.statements.insert(
        2,
        RawStatementSyntax {
            span: s(insertion, insertion + 14),
            kind: RawStatementKind::Assignment {
                target,
                equals_span: s(insertion + 8, insertion + 9),
                value,
                semicolon_span: s(insertion + 13, insertion + 14),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2, 3];
    (source, raw)
}

#[test]
fn projected_string_assignment_prepares_before_replacing_the_exact_leaf() {
    let (source, raw) = owned_pair_projected_string_assignment_snapshot(
        OwnedPairProjectedStringAssignmentRhs::Fresh,
        true,
    );
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected ReplacePlace");
    let replace = instructions[replace_index];
    let target = replace.place_operands().next().expect("projected replacement target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
    ));
    let prepared = instructions[..replace_index]
        .iter()
        .rev()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("prepared replacement String");
    assert!(
        prepared.derived_drop_actions().any(|action| action.root() == root),
        "fallible preparation retains the enclosing aggregate root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the replaced String leaf",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_string_clone_assignment_prepares_before_replacing_the_exact_leaf() {
    let (source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("source-faithful projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected StringClone");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected ReplacePlace");
    assert!(clone_index < replace_index, "clone preparation precedes assignment commit");
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source = clone.place_operands().next().expect("projected clone source");
    let target = replace.place_operands().next().expect("projected replacement target");
    assert_eq!(source, target, "self-clone reads the exact leaf replaced at commit");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [root],
        "fallible clone preparation retains the enclosing root",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the exact old leaf",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_string_clone_fault_trace_retains_the_enclosing_root() {
    let (source, raw) = owned_pair_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected clone fault");
    let program = lower(pair_input(&syntax, &sources)).expect("projected String clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected StringClone");
    let first = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("authenticated projected clone fault");
    let replay = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("deterministic projected clone fault");
    assert_eq!(first, replay);
    assert!(!first.result_committed);
    assert_eq!(first.uncommitted_result, clone.result());
    assert_eq!(first.retained_roots, [root]);
    assert_eq!(first.reverse_cleanup, [root]);
}

#[test]
fn fixed_array_projected_string_assignment_uses_the_same_exact_leaf_commit() {
    let (source, raw) = owned_array_projected_string_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("array String-leaf assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let replace = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("array projected ReplacePlace");
    let target = replace.place_operands().next().expect("array replacement target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == root
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
}

#[test]
fn fixed_array_projected_string_clone_reads_and_replaces_the_exact_element() {
    let (source, raw) = owned_array_projected_string_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful array clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("array String clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned array root")
        .id();
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let clone_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("projected array StringClone");
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("array projected ReplacePlace");
    assert!(clone_index < replace_index);
    let clone = instructions[clone_index];
    let replace = instructions[replace_index];
    let source = clone.place_operands().next().expect("array clone source");
    let target = replace.place_operands().next().expect("array replacement target");
    assert_eq!(source, target);
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == root
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [root],
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
}

#[test]
fn projected_string_assignment_rejects_immutable_and_self_consuming_targets() {
    for (rhs, mutable, needle, reference_ordinal, label) in [
        (OwnedPairProjectedStringAssignmentRhs::Fresh, false, "p.first", 0, "immutable projection"),
        (
            OwnedPairProjectedStringAssignmentRhs::TargetMove,
            true,
            "p",
            2,
            "self-consuming projection",
        ),
    ] {
        let (source, raw) = owned_pair_projected_string_assignment_snapshot(rhs, mutable);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful rejected projection");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(label);
        assert_eq!(diagnostics.len(), 1, "{label}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3014", "{label}");
        assert_eq!(
            diagnostics[0].primary_span(),
            Some(span(&sources, nth_untrusted_span(&source, needle, reference_ordinal))),
            "{label}",
        );
    }
}

#[test]
fn projected_string_assignment_does_not_reinitialize_a_moved_leaf() {
    let (source, raw) = owned_pair_moved_projection_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful moved projection");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("moved projection target");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "p.first", 1))),
    );
}

#[test]
fn projected_string_assignment_rejects_a_copy_leaf_target_as_m3013() {
    let (source, raw) = owned_pair_copy_projection_assignment_target_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Copy target");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("Copy target shape");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "p.flag", 0))),
    );
}
