use super::*;

#[test]
fn conditional_root_borrows_use_canonical_blocks_and_discharge_each_arm() {
    let sources = sources_for(CONDITIONAL_ROOT_BORROW_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(CONDITIONAL_ROOT_BORROW_JSON).expect("conditional borrow snapshot"),
        &sources,
    )
    .expect("source-faithful conditional borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("conditional borrow lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(function.places().count(), 1, "arm-local Copy reads do not enter join state");
    assert_eq!(
        blocks.iter().flat_map(|block| block.instructions()).count(),
        15,
        "preflighted transition total must match materialized raw IR",
    );
    assert_eq!(
        blocks
            .iter()
            .flat_map(|block| block.instructions())
            .filter(|instruction| instruction.result().is_some())
            .count(),
        7,
        "root, condition, three reads, one write literal, and final return",
    );
    assert_eq!(blocks.iter().map(|block| block.id().index()).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    assert!(blocks.iter().all(|block| block.parameters().next().is_none()));
    assert_eq!(
        blocks.iter().map(|block| block.terminator().kind()).collect::<Vec<_>>(),
        vec![
            VerifiedTerminatorKind::Branch,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Jump,
            VerifiedTerminatorKind::Return,
        ]
    );
    let branch = blocks[0].terminator();
    let (when_true, when_false) = branch.branch_edges().expect("canonical branch edges");
    assert_eq!((when_true.target().index(), when_false.target().index()), (1, 2));
    assert_eq!(when_true.arguments().count(), 0);
    assert_eq!(when_false.arguments().count(), 0);
    let then = blocks[1].instructions().collect::<Vec<_>>();
    assert_eq!(
        then.iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .map(|instruction| instruction.borrow().expect("then borrow").index())
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        then.iter()
            .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
            .map(|instruction| instruction.borrow().expect("then end").index())
            .collect::<Vec<_>>(),
        vec![1, 0]
    );
    let otherwise = blocks[2].instructions().collect::<Vec<_>>();
    assert_eq!(
        otherwise
            .iter()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("exclusive else borrow")
            .borrow()
            .expect("else borrow identity")
            .index(),
        2
    );
    assert!(otherwise.iter().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::BeginBorrow
            && instruction.borrow_access() == Some(VerifiedBorrowAccess::Exclusive)
    }));
    assert_eq!(
        otherwise.last().expect("else lexical end").kind(),
        VerifiedInstructionKind::EndBorrow
    );
    assert_eq!(blocks[3].instructions().count(), 1);
    assert_eq!(
        blocks[3].instructions().next().expect("joined owner read").kind(),
        VerifiedInstructionKind::CopyFromPlace
    );
}

#[test]
fn conditional_root_borrow_accepts_one_complete_arm_and_empty_peer_scope() {
    let sources = sources_for(CONDITIONAL_ROOT_BORROW_ONE_ARM_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(CONDITIONAL_ROOT_BORROW_ONE_ARM_JSON).expect("one-arm snapshot"),
        &sources,
    )
    .expect("source-faithful one-arm v4");
    let program = lower(pair_input(&syntax, &sources)).expect("complete one-arm borrow");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert!(
        blocks[1]
            .instructions()
            .any(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
    );
    assert_eq!(blocks[2].instructions().count(), 0);
    assert_eq!(blocks[1].terminator().kind(), VerifiedTerminatorKind::Jump);
    assert_eq!(blocks[2].terminator().kind(), VerifiedTerminatorKind::Jump);
}

#[test]
fn conditional_root_borrow_accepts_empty_then_and_complete_else_arm() {
    let sources = sources_for(CONDITIONAL_ROOT_BORROW_ELSE_ONLY_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(CONDITIONAL_ROOT_BORROW_ELSE_ONLY_JSON).expect("else-only snapshot"),
        &sources,
    )
    .expect("source-faithful else-only v4");
    let program = lower(pair_input(&syntax, &sources)).expect("complete else-arm borrow");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks[1].instructions().count(), 0);
    assert!(
        blocks[2]
            .instructions()
            .any(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
    );
}

#[test]
fn conditional_root_borrow_exclusions_are_source_faithful_and_stable() {
    let sources = sources_for(CONDITIONAL_ROOT_BORROW_EXCLUSIONS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(CONDITIONAL_ROOT_BORROW_EXCLUSIONS_JSON).expect("exclusion snapshot"),
        &sources,
    )
    .expect("source-faithful conditional exclusions v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("excluded conditional forms");
    assert_eq!(diagnostics.len(), 8);

    let source = CONDITIONAL_ROOT_BORROW_EXCLUSIONS_SOURCE;
    let repeated_body = source.find("function repeatedConditional").expect("repeated function")
        + source[source.find("function repeatedConditional").expect("repeated function")..]
            .find('{')
            .expect("repeated body");
    let expected = [
        (
            "ZRYNA-M3017",
            "conditional borrowing requires an explicit else arm",
            source.find("if (missingRoot)").expect("missing-else conditional"),
            None,
        ),
        (
            "ZRYNA-M3017",
            "conditional borrow arms require exactly one explicit lexical scope",
            source.find("{\n    const directAlias").expect("direct-arm block"),
            None,
        ),
        (
            "ZRYNA-M3008",
            "this statement form is outside deterministic aggregate M3",
            source.find("if (emptyRoot)").expect("empty conditional"),
            Some(588_u32),
        ),
        (
            "ZRYNA-M3017",
            "conditional borrowing requires the exact bool root as its condition",
            source.find("false").expect("literal condition"),
            None,
        ),
        (
            "ZRYNA-M3017",
            "conditional borrowing requires one literal-initialized bool root",
            source.find("const numberRoot").expect("non-bool root"),
            None,
        ),
        (
            "ZRYNA-M3017",
            "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes",
            source.find("      if (nestedRoot)").expect("nested conditional") + 6,
            None,
        ),
        (
            "ZRYNA-M3017",
            "shared-root borrowing requires one root local, one lexical block, and one final return",
            repeated_body,
            None,
        ),
        (
            "ZRYNA-M3017",
            "root-borrow blocks admit only const aliases, const Copy reads, and BorrowMut writes",
            source.find("return returnRoot;").expect("arm-local return"),
            None,
        ),
    ];
    for (index, (diagnostic, (code, message, start, end))) in
        diagnostics.iter().zip(expected).enumerate()
    {
        let primary = diagnostic.primary_span().map(|span| (span.start(), span.end()));
        assert_eq!(diagnostic.code(), code, "diagnostic {index}: {diagnostic:?}");
        assert_eq!(diagnostic.message(), message, "diagnostic {index}: {diagnostic:?}");
        assert_eq!(
            primary.map(|(start, _)| start),
            u32::try_from(start).ok(),
            "diagnostic {index}: {diagnostic:?}"
        );
        if let Some(end) = end {
            assert_eq!(
                primary.map(|(_, end)| end),
                Some(end),
                "diagnostic {index}: {diagnostic:?}"
            );
        }
    }
}

#[test]
fn conditional_root_borrow_accepts_exclusive_authority_in_both_arms() {
    let sources = sources_for(CONDITIONAL_ROOT_BORROW_EXCLUSIVE_ARMS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(CONDITIONAL_ROOT_BORROW_EXCLUSIVE_ARMS_JSON)
            .expect("exclusive-arms snapshot"),
        &sources,
    )
    .expect("source-faithful exclusive-arms v4");
    let program = lower(pair_input(&syntax, &sources)).expect("exclusive arm-local borrows");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    for (block, borrow) in [(blocks[1], 0), (blocks[2], 1)] {
        let instructions = block.instructions().collect::<Vec<_>>();
        assert!(instructions.iter().any(|instruction| {
            instruction.kind() == VerifiedInstructionKind::BeginBorrow
                && instruction.borrow_access() == Some(VerifiedBorrowAccess::Exclusive)
                && instruction.borrow().expect("exclusive borrow").index() == borrow
        }));
        assert!(
            instructions
                .iter()
                .any(|instruction| instruction.kind() == VerifiedInstructionKind::BorrowWrite)
        );
        assert_eq!(
            instructions.last().expect("arm end").kind(),
            VerifiedInstructionKind::EndBorrow
        );
    }
}

#[test]
fn conditional_arm_conflicts_and_owner_access_fail_before_ir_construction() {
    for (source, snapshot, expected) in [
        (
            CONDITIONAL_ROOT_BORROW_CONFLICT_SOURCE,
            CONDITIONAL_ROOT_BORROW_CONFLICT_JSON,
            "borrow access conflicts with an active alias of the same root",
        ),
        (
            CONDITIONAL_ROOT_BORROW_OWNER_READ_SOURCE,
            CONDITIONAL_ROOT_BORROW_OWNER_READ_JSON,
            "owner reads are hidden while an exclusive alias is active",
        ),
    ] {
        let sources = sources_for(source);
        let syntax = verify_snapshot(
            decode_snapshot(snapshot).expect("hostile conditional snapshot"),
            &sources,
        )
        .expect("source-faithful hostile conditional v4");
        let diagnostics = lower(pair_input(&syntax, &sources)).expect_err(expected);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3017");
        assert_eq!(diagnostics[0].message(), expected);
    }
}

#[test]
fn conditional_root_borrow_lowering_is_deterministic() {
    let trace = || {
        let sources = sources_for(CONDITIONAL_ROOT_BORROW_SOURCE);
        let syntax = verify_snapshot(
            decode_snapshot(CONDITIONAL_ROOT_BORROW_JSON).expect("conditional replay snapshot"),
            &sources,
        )
        .expect("source-faithful conditional replay v4");
        let program = lower(pair_input(&syntax, &sources)).expect("conditional replay lowering");
        program
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .map(|block| {
                (
                    block.id().index(),
                    block
                        .instructions()
                        .map(|instruction| {
                            (
                                instruction.kind(),
                                instruction
                                    .borrow()
                                    .map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
                                instruction.borrow_access(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    block.terminator().kind(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(trace(), trace());
}

#[test]
fn conditional_root_borrow_active_peak_is_per_arm_not_summed() {
    let exact = zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION;
    assert_eq!(conditional_root_borrow_budget_violation(exact, 0, 0, exact, 0, 0), None);
    assert_eq!(
        conditional_root_borrow_budget_violation(exact + 1, 0, 0, 0, 0, 0),
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    );
}

#[test]
fn conditional_root_borrow_resources_sum_arm_costs_and_saturate() {
    let resources = conditional_root_borrow_resources(2, 3, 5, 7, 11, 13);
    assert_eq!(resources.values, 35, "fourteen reads, eighteen writes, and three fixed values");
    assert_eq!(resources.places, 1);
    assert_eq!(
        resources.transitions, 72,
        "nine aliases, arm reads/writes, and four fixed transitions"
    );
    assert_eq!(resources.blocks, 4);
    assert_eq!(resources.edges, 4);
    assert_eq!(resources.active_peak, 7, "active capacity is not the nine-alias arm sum");
    assert_eq!(resources.cleanup_plans, 1);

    let saturated = conditional_root_borrow_resources(usize::MAX, usize::MAX, usize::MAX, 1, 1, 1);
    assert_eq!(saturated.values, usize::MAX);
    assert_eq!(saturated.transitions, usize::MAX);
    assert_eq!(saturated.active_peak, usize::MAX);
    assert_eq!(
        conditional_root_borrow_budget_violation(usize::MAX, usize::MAX, usize::MAX, 1, 1, 1),
        Some(RootBorrowBudgetLimit::Values),
    );
}

#[test]
fn conditional_root_borrow_resources_obey_exact_and_first_extra_limits() {
    let base = conditional_root_borrow_resources(1, 1, 0, 1, 1, 0);
    for (limit, exact, extra) in [
        (
            RootBorrowBudgetLimit::Values,
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
                ..base
            },
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION + 1,
                ..base
            },
        ),
        (
            RootBorrowBudgetLimit::Places,
            RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
                ..base
            },
            RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION + 1,
                ..base
            },
        ),
        (
            RootBorrowBudgetLimit::Transitions,
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                ..base
            },
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    + 1,
                ..base
            },
        ),
    ] {
        assert_eq!(root_borrow_resource_violation(exact), None);
        assert_eq!(root_borrow_resource_violation(extra), Some(limit));
    }
}

#[test]
fn root_borrow_resources_enforce_exact_block_and_edge_limits() {
    assert_eq!(
        root_borrow_resource_violation(RootBorrowResources {
            blocks: zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION,
            edges: zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION,
            ..RootBorrowResources::default()
        }),
        None,
    );
    assert_eq!(
        root_borrow_resource_violation(RootBorrowResources {
            blocks: zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION + 1,
            ..RootBorrowResources::default()
        }),
        Some(RootBorrowBudgetLimit::Blocks),
    );
    assert_eq!(
        root_borrow_resource_violation(RootBorrowResources {
            edges: zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION + 1,
            ..RootBorrowResources::default()
        }),
        Some(RootBorrowBudgetLimit::Edges),
    );
}

#[test]
fn root_borrow_resources_enforce_exact_value_place_and_transition_limits() {
    for (limit, exact, extra) in [
        (
            RootBorrowBudgetLimit::Values,
            root_borrow_resource_violation(RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
                ..RootBorrowResources::default()
            }),
            root_borrow_resource_violation(RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION + 1,
                ..RootBorrowResources::default()
            }),
        ),
        (
            RootBorrowBudgetLimit::Places,
            root_borrow_resource_violation(RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
                ..RootBorrowResources::default()
            }),
            root_borrow_resource_violation(RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION + 1,
                ..RootBorrowResources::default()
            }),
        ),
        (
            RootBorrowBudgetLimit::Transitions,
            root_borrow_resource_violation(RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                ..RootBorrowResources::default()
            }),
            root_borrow_resource_violation(RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    + 1,
                ..RootBorrowResources::default()
            }),
        ),
    ] {
        assert_eq!(exact, None);
        assert_eq!(extra, Some(limit));
    }
}
