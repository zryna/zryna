use super::*;

const SHARED_SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-shared.zry");
const SHARED_JSON: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-shared.json");
const EXCLUSIVE_SOURCE: &str =
    include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-exclusive.zry");
const EXCLUSIVE_JSON: &[u8] =
    include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-exclusive.json");

const REJECTIONS: [(&str, &[u8], &str); 9] = [
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-inactive.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-inactive.json"),
        "borrow alias 'loan' is not active at this call",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-borrow-kind.zry"),
        include_bytes!(
            "../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-borrow-kind.json"
        ),
        "borrow call arguments must name one active lexical alias",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-value-kind.zry"),
        include_bytes!(
            "../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-value-kind.json"
        ),
        "borrow authority cannot satisfy a by-value call parameter",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-access.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-access.json"),
        "lexical borrow argument does not match the callee referent and access",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-referent.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-referent.json"),
        "lexical borrow argument does not match the callee referent and access",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-arity.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-wrong-arity.json"),
        "call to 'relay' has 2 arguments but its signature requires 3",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-projected.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-projected.json"),
        "projected lexical borrows cannot be passed to calls",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-cfg.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-cfg.json"),
        "lexical borrow calls cannot cross a control-flow edge",
    ),
    (
        include_str!("../../../../../tests/m3-fixtures/lexical-borrow-call-repeated.zry"),
        include_bytes!("../../../../../tests/m3-fixtures/lexical-borrow-call-repeated.json"),
        "a lexical borrow block admits only one direct call",
    ),
];

fn with_fixture<T>(
    source: &str,
    json: &[u8],
    inspect: impl FnOnce(VerifiedFunction<'_>) -> T,
) -> Result<T, Vec<zryna_diagnostics::Diagnostic>> {
    let sources = sources_for(source);
    let raw = decode_snapshot(json).expect("lexical borrow-call fixture");
    let syntax =
        verify_snapshot(raw, &sources).expect("source-faithful lexical borrow-call fixture");
    let program = lower(pair_input(&syntax, &sources))?;
    let caller = program.modules().next().expect("module").functions().nth(1).expect("caller");
    Ok(inspect(caller))
}

fn assert_lexical_call(source: &str, json: &[u8], access: VerifiedBorrowAccess) {
    with_fixture(source, json, |caller| {
        let instructions =
            caller.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
            [
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::BeginBorrow,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::I32Literal,
                VerifiedInstructionKind::DirectCall,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::EndBorrow,
                VerifiedInstructionKind::CopyFromPlace,
            ]
        );
        let begin = instructions[2];
        let call = instructions[5];
        let end = instructions[7];
        assert_eq!(begin.borrow_access(), Some(access));
        assert_eq!(begin.borrow().expect("begin").index(), 0);
        assert_eq!(end.borrow().expect("end").index(), 0);
        let arguments = call.call_arguments().collect::<Vec<_>>();
        assert!(matches!(arguments[0], VerifiedCallArgument::Value(value) if value.index() == 1));
        assert!(matches!(arguments[1], VerifiedCallArgument::Value(value) if value.index() == 2));
        assert!(
            matches!(arguments[2], VerifiedCallArgument::Borrow(borrow) if borrow.index() == 0)
        );
    })
    .expect("lexical borrow call");
}

#[test]
fn shared_lexical_borrow_call_is_canonical_and_caller_ended() {
    assert_lexical_call(SHARED_SOURCE, SHARED_JSON, VerifiedBorrowAccess::Shared);
}

#[test]
fn lexical_borrow_call_evaluates_values_in_left_to_right_source_order() {
    with_fixture(SHARED_SOURCE, SHARED_JSON, |caller| {
        assert_eq!(
            caller
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .filter_map(zryna_ir::data_ownership_v1::VerifiedInstruction::i32_literal)
                .collect::<Vec<_>>(),
            [7, 11, 22]
        );
    })
    .expect("source-ordered lexical borrow call");
}

#[test]
fn lexical_borrow_call_owns_one_unique_call_trap_cleanup() {
    with_fixture(SHARED_SOURCE, SHARED_JSON, |caller| {
        let block = caller.blocks().next().expect("block");
        let call = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .expect("direct call");
        let cleanup = call.cleanup().expect("CallTrap cleanup");
        let plan = caller.cleanup_plans().find(|plan| plan.id() == cleanup).expect("cleanup plan");
        assert_eq!(plan.site().role(), VerifiedCleanupRole::CallTrap);
        assert_eq!(plan.actions().count(), 0);
        assert_eq!(
            caller
                .cleanup_plans()
                .filter(|plan| plan.site().role() == VerifiedCleanupRole::CallTrap)
                .count(),
            1
        );
        assert_ne!(cleanup, block.terminator().cleanup().expect("return cleanup"));
    })
    .expect("cleanup-owned lexical borrow call");
}

#[test]
fn lexical_borrow_call_resource_formula_is_exact_and_saturating() {
    let exact = straight_root_borrow_resources(1, 0, 0, 1, 2);
    assert_eq!(exact.values, 5);
    assert_eq!(exact.places, 2);
    assert_eq!(exact.transitions, 9);
    assert_eq!(exact.blocks, 1);
    assert_eq!(exact.edges, 0);
    assert_eq!(exact.active_peak, 1);
    assert_eq!(exact.cleanup_plans, 2);
    let saturated =
        straight_root_borrow_resources(usize::MAX, usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    assert_eq!(saturated.values, usize::MAX);
    assert_eq!(saturated.places, usize::MAX);
    assert_eq!(saturated.transitions, usize::MAX);
    assert_eq!(saturated.cleanup_plans, usize::MAX);
    assert_eq!(root_borrow_resource_violation(saturated), Some(RootBorrowBudgetLimit::Values));
}

#[test]
fn borrow_call_resource_preflight_accepts_exact_limits_and_rejects_first_extra_in_order() {
    let additional = RootBorrowResources {
        values: 1,
        places: 1,
        transitions: 1,
        blocks: 1,
        edges: 1,
        active_peak: 1,
        cleanup_plans: 1,
    };
    let exact = RootBorrowResources {
        values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - 1,
        places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - 1,
        transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1,
        blocks: zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION - 1,
        edges: zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION - 1,
        active_peak: zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION - 1,
        cleanup_plans: zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
    };
    let reserved = checked_add_resources(exact, additional).expect("exact borrow-call boundary");
    assert_eq!(reserved.values, zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION);
    assert_eq!(reserved.places, zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION);
    assert_eq!(
        reserved.transitions,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
    );
    assert_eq!(reserved.blocks, zryna_ir::data_ownership_v1::MAX_BLOCKS_PER_FUNCTION);
    assert_eq!(reserved.edges, zryna_ir::data_ownership_v1::MAX_CFG_EDGES_PER_FUNCTION);
    assert_eq!(reserved.active_peak, zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION);
    assert_eq!(reserved.cleanup_plans, zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION);

    let cases = [
        (RootBorrowResources { values: exact.values + 1, ..exact }, RootBorrowBudgetLimit::Values),
        (RootBorrowResources { places: exact.places + 1, ..exact }, RootBorrowBudgetLimit::Places),
        (
            RootBorrowResources { transitions: exact.transitions + 1, ..exact },
            RootBorrowBudgetLimit::Transitions,
        ),
        (RootBorrowResources { blocks: exact.blocks + 1, ..exact }, RootBorrowBudgetLimit::Blocks),
        (RootBorrowResources { edges: exact.edges + 1, ..exact }, RootBorrowBudgetLimit::Edges),
        (
            RootBorrowResources { active_peak: exact.active_peak + 1, ..exact },
            RootBorrowBudgetLimit::ActiveBorrows,
        ),
        (
            RootBorrowResources { cleanup_plans: exact.cleanup_plans + 1, ..exact },
            RootBorrowBudgetLimit::CleanupPlans,
        ),
    ];
    for (current, expected) in cases {
        let before = current;
        assert!(matches!(
            checked_add_resources(current, additional),
            Err(BorrowCallPreflightError::Limit(actual)) if actual == expected
        ));
        assert_eq!(current.values, before.values);
        assert_eq!(current.places, before.places);
        assert_eq!(current.transitions, before.transitions);
        assert_eq!(current.blocks, before.blocks);
        assert_eq!(current.edges, before.edges);
        assert_eq!(current.active_peak, before.active_peak);
        assert_eq!(current.cleanup_plans, before.cleanup_plans);
    }
}

#[test]
fn borrow_call_resource_overflow_precedes_limit_selection_and_preserves_authority_cost() {
    let current = RootBorrowResources {
        values: usize::MAX,
        places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
        ..RootBorrowResources::default()
    };
    assert!(matches!(
        checked_add_resources(current, RootBorrowResources { values: 1, ..Default::default() }),
        Err(BorrowCallPreflightError::Overflow)
    ));
    assert!(
        checked_straight_borrow_call_resources(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX
        )
        .is_none()
    );

    let forwarded = checked_call_delta(RootBorrowResources::default(), false)
        .expect("forwarded borrow call delta");
    assert_eq!(forwarded.values, 1);
    assert_eq!(forwarded.places, 0);
    assert_eq!(forwarded.transitions, 1);
    assert_eq!(forwarded.active_peak, 0);
    assert_eq!(forwarded.cleanup_plans, 1);
    let lexical = checked_straight_borrow_call_resources(1, 0, 0, 1, 0)
        .expect("lexical borrow-call resources");
    assert_eq!(lexical.active_peak, 1);
}

#[test]
fn borrow_call_program_edge_and_depth_boundaries_are_exact() {
    assert_eq!(
        borrow_call_program_budget_violation(
            zryna_ir::data_ownership_v1::MAX_CALL_EDGES,
            zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH,
        ),
        None
    );
    assert_eq!(
        borrow_call_program_budget_violation(
            zryna_ir::data_ownership_v1::MAX_CALL_EDGES + 1,
            zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH + 1,
        ),
        Some(BorrowCallProgramBudgetLimit::CallEdges)
    );
    assert_eq!(
        borrow_call_program_budget_violation(
            zryna_ir::data_ownership_v1::MAX_CALL_EDGES,
            zryna_ir::data_ownership_v1::MAX_STATIC_CALL_DEPTH + 1,
        ),
        Some(BorrowCallProgramBudgetLimit::CallDepth)
    );
}

#[test]
fn exclusive_lexical_borrow_call_preserves_exact_authority_and_caller_end() {
    assert_lexical_call(EXCLUSIVE_SOURCE, EXCLUSIVE_JSON, VerifiedBorrowAccess::Exclusive);
}

#[test]
fn rejected_lexical_call_shapes_are_atomic_and_do_not_contaminate_later_lowering() {
    let baseline = with_fixture(SHARED_SOURCE, SHARED_JSON, |caller| {
        caller
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .map(|instruction| (instruction.kind(), instruction.i32_literal()))
            .collect::<Vec<_>>()
    })
    .expect("baseline");
    for (source, json, message) in REJECTIONS {
        let diagnostics = with_fixture(source, json, |_| ()).expect_err(message);
        assert_eq!(diagnostics.len(), 1, "{message}");
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3016", "{message}");
        assert_eq!(diagnostics[0].message(), message);

        let replay = with_fixture(SHARED_SOURCE, SHARED_JSON, |caller| {
            caller
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .map(|instruction| (instruction.kind(), instruction.i32_literal()))
                .collect::<Vec<_>>()
        })
        .expect("uncontaminated replay");
        assert_eq!(replay, baseline, "rejected planning must not retain IR or resource state");
    }
}
