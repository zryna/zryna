use super::*;

#[test]
fn owned_root_shared_reads_reuse_existing_operations_and_restore_each_owner() {
    let sources = sources_for(OWNED_ROOT_BORROW_READS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(OWNED_ROOT_BORROW_READS_JSON).expect("owned-root borrow snapshot"),
        &sources,
    )
    .expect("source-faithful owned-root borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned-root shared reads");
    let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
    assert_eq!(functions.len(), 6);

    let expected_reads = [
        vec![VerifiedInstructionKind::StringClone, VerifiedInstructionKind::StringConcat],
        vec![VerifiedInstructionKind::VecIndexCopy, VerifiedInstructionKind::VecIndexCopy],
        vec![VerifiedInstructionKind::VecIndexCopy, VerifiedInstructionKind::VecIndexCopy],
        vec![VerifiedInstructionKind::ClonePlace],
        vec![VerifiedInstructionKind::ClonePlace],
        vec![VerifiedInstructionKind::ClonePlace],
    ];
    let expected_owned_drops = [2_usize, 0, 0, 1, 1, 1];
    for ((function, expected), expected_owned_drops) in
        functions.iter().zip(expected_reads).zip(expected_owned_drops)
    {
        assert_eq!(function.borrow_parameters().count(), 0);
        let block = function.blocks().next().expect("single block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let begin = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("shared begin");
        let end = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("lexical end");
        let final_move = instructions
            .iter()
            .rposition(|instruction| instruction.kind() == VerifiedInstructionKind::MoveFromPlace)
            .expect("root move after lexical end");
        assert_eq!(end + 1, final_move);
        assert_eq!(instructions[begin].borrow_access(), Some(VerifiedBorrowAccess::Shared));
        assert_eq!(instructions[begin].borrow().expect("borrow identity").index(), 0);
        assert_eq!(instructions[end].borrow().expect("ended identity").index(), 0);
        let root = instructions[begin].place_operands().next().expect("borrowed root");
        assert_eq!(instructions[final_move].place_operands().next().expect("returned root"), root);
        let reads = instructions
            .iter()
            .enumerate()
            .filter(|(_, instruction)| expected.contains(&instruction.kind()))
            .collect::<Vec<_>>();
        assert_eq!(
            reads.iter().map(|(_, instruction)| instruction.kind()).collect::<Vec<_>>(),
            expected
        );
        assert!(reads.iter().all(|(index, instruction)| {
            *index > begin
                && *index < end
                && instruction.place_operands().all(|operand| operand == root)
        }));
        assert!(reads.iter().all(|(_, instruction)| {
            instruction.derived_drop_actions().any(|action| action.root() == root)
        }));
        for (_, instruction) in reads
            .iter()
            .filter(|(_, instruction)| instruction.kind() != VerifiedInstructionKind::VecIndexCopy)
        {
            let result = instruction.result().expect("owned read result");
            let owner = function
                .places()
                .find(|place| place.kind() == VerifiedPlaceKind::Temporary(result))
                .expect("distinct temporary owner for owned read result");
            assert_ne!(owner.id(), root);
        }
        let lexical_drops = instructions
            .iter()
            .enumerate()
            .filter(|(_, instruction)| instruction.kind() == VerifiedInstructionKind::DropPlace)
            .map(|(index, instruction)| {
                assert!(index < end, "owned result must drop before EndBorrow");
                let place = instruction.place_operands().next().expect("lexical drop place");
                assert_ne!(place, root);
                place
            })
            .collect::<Vec<_>>();
        assert_eq!(lexical_drops.len(), expected_owned_drops);
        assert!(
            lexical_drops.windows(2).all(|places| places[0].index() > places[1].index()),
            "owned lexical locals must drop in reverse declaration order"
        );
        let return_cleanup = block.terminator().cleanup().expect("return cleanup identity");
        assert_eq!(
            function
                .cleanup_plans()
                .find(|plan| plan.id() == return_cleanup)
                .expect("return cleanup plan")
                .actions()
                .count(),
            0,
            "lexically dropped read results cannot survive into return cleanup"
        );
    }
}

#[test]
fn owned_root_borrow_faults_retain_the_source_and_exact_cleanup_authority() {
    let sources = sources_for(OWNED_ROOT_BORROW_READS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(OWNED_ROOT_BORROW_READS_JSON).expect("owned-root fault snapshot"),
        &sources,
    )
    .expect("source-faithful owned-root fault v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned-root fault authority");
    let abi = program.runtime_abi();
    let mut covered = Vec::new();
    for function in program.modules().next().expect("module").functions() {
        let block = function.blocks().next().expect("single block");
        let root = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
            .and_then(|instruction| instruction.place_operands().next())
            .expect("borrowed root");
        for instruction in block.instructions().filter(|instruction| {
            matches!(
                instruction.kind(),
                VerifiedInstructionKind::StringClone
                    | VerifiedInstructionKind::StringConcat
                    | VerifiedInstructionKind::VecIndexCopy
                    | VerifiedInstructionKind::ClonePlace
            )
        }) {
            let injection = match instruction.kind() {
                VerifiedInstructionKind::StringClone => OwnedFaultInjection::Runtime {
                    operation: LogicalOperation::StringClone,
                    status: RuntimeStatus::Allocation,
                },
                VerifiedInstructionKind::StringConcat => OwnedFaultInjection::Runtime {
                    operation: LogicalOperation::StringConcat,
                    status: RuntimeStatus::Allocation,
                },
                VerifiedInstructionKind::VecIndexCopy => OwnedFaultInjection::Bounds,
                VerifiedInstructionKind::ClonePlace => OwnedFaultInjection::AggregateCloneElement {
                    status: RuntimeStatus::Allocation,
                    completed_prefix: 0,
                },
                _ => unreachable!("filtered owned-root read"),
            };
            let trace = owned_fault_trace(abi, function, instruction, injection, 0, 8)
                .expect("authenticated owned-root failure trace");
            assert!(!trace.result_committed);
            assert!(trace.retained_roots.contains(&root));
            assert!(trace.reverse_cleanup.contains(&root));
            covered.push(instruction.kind());
        }
    }
    assert_eq!(
        covered,
        [
            VerifiedInstructionKind::StringClone,
            VerifiedInstructionKind::StringConcat,
            VerifiedInstructionKind::VecIndexCopy,
            VerifiedInstructionKind::VecIndexCopy,
            VerifiedInstructionKind::VecIndexCopy,
            VerifiedInstructionKind::VecIndexCopy,
            VerifiedInstructionKind::ClonePlace,
            VerifiedInstructionKind::ClonePlace,
            VerifiedInstructionKind::ClonePlace,
        ]
    );
}

#[test]
fn owned_root_borrow_authority_budget_is_exact_saturating_and_atomic() {
    let exact_transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 4;
    assert_eq!(owned_root_borrow_resource_violation(exact_transitions, 2, 0), None);
    assert_eq!(
        owned_root_borrow_resource_violation(exact_transitions + 1, 2, 0),
        Some(RootBorrowBudgetLimit::Transitions)
    );
    let exact_active = zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION - 1;
    assert_eq!(owned_root_borrow_resource_violation(0, 0, exact_active), None);
    assert_eq!(
        owned_root_borrow_resource_violation(0, 0, exact_active + 1),
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    );
    assert_eq!(
        owned_root_borrow_resource_violation(usize::MAX, usize::MAX, usize::MAX),
        Some(RootBorrowBudgetLimit::Transitions)
    );
}

#[test]
fn owned_root_borrow_routing_is_exactly_whole_root_and_shared() {
    let sources = sources_for(OWNED_ROOT_BORROW_READS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(OWNED_ROOT_BORROW_READS_JSON).expect("owned-root routing snapshot"),
        &sources,
    )
    .expect("source-faithful owned-root routing v4");
    let file = &syntax.files()[0];
    assert!(
        file.functions()
            .iter()
            .all(|function| is_direct_owned_root_borrow_candidate(file, function))
    );

    let sources = sources_for(PROJECTED_BORROW_EXCLUSIONS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(PROJECTED_BORROW_EXCLUSIONS_JSON).expect("projection exclusions snapshot"),
        &sources,
    )
    .expect("source-faithful projection exclusions v4");
    let file = &syntax.files()[0];
    for name in ["vecElement", "enumPayload", "nonCopyProjection"] {
        let function = file
            .functions()
            .iter()
            .find(|function| function.name.text == name)
            .expect("named projection exclusion function");
        assert!(!is_direct_owned_root_borrow_candidate(file, function), "{name}");
    }
}

#[test]
fn owned_root_borrow_exclusions_are_ordered_source_faithful_and_deterministic() {
    let sources = sources_for(OWNED_ROOT_BORROW_EXCLUSIONS_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(OWNED_ROOT_BORROW_EXCLUSIONS_JSON).expect("owned-root exclusion snapshot"),
        &sources,
    )
    .expect("source-faithful owned-root exclusions v4");
    let lower_once = || {
        lower(pair_input(&syntax, &sources))
            .expect_err("owned-root exclusions")
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_owned(),
                    diagnostic.primary_span().map(|span| (span.start(), span.end())),
                    diagnostic.message().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    let first = lower_once();
    assert_eq!(first, lower_once());
    assert_eq!(
        first,
        [
            (
                "ZRYNA-M3017".to_owned(),
                Some((59, 242)),
                "root borrowing requires an exact recursively Copy result".to_owned(),
            ),
            (
                "ZRYNA-M3017".to_owned(),
                Some((244, 451)),
                "root borrowing requires an exact recursively Copy result".to_owned(),
            ),
            (
                "ZRYNA-M3017".to_owned(),
                Some((601, 606)),
                "operation is outside whole owned-root shared reads".to_owned(),
            ),
            (
                "ZRYNA-M3017".to_owned(),
                Some((758, 779)),
                "owned-root borrow blocks admit only const read results".to_owned(),
            ),
            (
                "ZRYNA-M3017".to_owned(),
                Some((980, 988)),
                "operation is outside whole owned-root shared reads".to_owned(),
            ),
        ]
    );
}
