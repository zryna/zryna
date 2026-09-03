use super::*;

use super::owned_root_borrow_boundary_fixture_support::owned_root_cleanup_boundary_fixture;
use zryna_ir::data_ownership_v1::{BorrowIdentity, CleanupPlanIdentity};

fn cleanup_boundary_observation(program: &super::super::VerifiedProgram) -> Vec<String> {
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let mut observation = Vec::new();
    for block in function.blocks() {
        for instruction in block.instructions() {
            observation.push(format!(
                "{:?}/{:?}/{:?}/{:?}/{:?}/{:?}",
                instruction.kind(),
                instruction.result().map(FaultValueIdentity::index),
                instruction.place_operands().map(FaultPlaceIdentity::index).collect::<Vec<_>>(),
                instruction.value_operands().map(FaultValueIdentity::index).collect::<Vec<_>>(),
                instruction.cleanup().map(CleanupPlanIdentity::index),
                instruction.borrow().map(BorrowIdentity::index),
            ));
        }
    }
    for cleanup in function.cleanup_plans() {
        observation.push(format!(
            "{:?}",
            cleanup.actions().map(FaultPlaceIdentity::index).collect::<Vec<_>>()
        ));
    }
    observation
}

#[test]
#[ignore = "authenticated exact/first-extra cleanup boundary runs in the full M3 preflight gate"]
#[allow(clippy::too_many_lines)]
fn owned_root_shared_read_drop_budget_is_authenticated_exact_and_first_extra() {
    let exact = owned_root_cleanup_boundary_fixture(17, 27);
    let extra = owned_root_cleanup_boundary_fixture(20, 25);
    for (fixture, expressions, operands) in [(&exact, 1_070, 46), (&extra, 1_071, 47)] {
        assert_eq!(fixture.source_expressions, expressions);
        assert_eq!(fixture.source_statements, 514);
        assert_eq!(fixture.source_types, 522);
        assert_eq!(fixture.construction_operands, operands);
        assert_eq!(fixture.raw.files[0].functions[0].body.blocks.len(), 2);
        assert_eq!(derived_value_count(&fixture.raw.files[0].functions[0]), expressions - 512);
        assert!(expressions - 512 < zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION);
        assert!(fixture.raw.files[0].functions[0].body.expressions.iter().all(|expression| {
            match &expression.kind {
                zryna_syntax::v4::RawExpressionKind::StringLiteral { spelling } => {
                    spelling == "\"\""
                }
                _ => true,
            }
        }));
        assert!(fixture.source.len() < zryna_syntax::v4::MAX_AGGREGATE_SOURCE_BYTES);
        assert!(expressions < zryna_syntax::v4::MAX_EXPRESSIONS_PER_FUNCTION);
        assert!(fixture.source_statements < zryna_syntax::v4::MAX_STATEMENTS_PER_FUNCTION);
        assert!(fixture.source_types < zryna_syntax::v4::MAX_TYPE_NODES_PER_MODULE);
        assert!(operands < zryna_syntax::v4::MAX_AGGREGATE_OPERANDS_PER_PROJECT);
    }
    // Constructor cleanup is A(A-1)/2 + B(B+1)/2. Clone k adds 2k+1
    // actions; the final return adds 510, later moved into explicit lexical drops.
    let clone_actions = (1..=510).map(|pending| 2 * pending + 1).sum::<usize>();
    assert_eq!(clone_actions, 261_120);
    let limit = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert_eq!(17 * 16 / 2 + 27 * 28 / 2 + clone_actions + 510, limit);
    assert_eq!(20 * 19 / 2 + 25 * 26 / 2 + clone_actions + 510, limit + 1);

    let sources = sources_for(&exact.source);
    let syntax = verify_snapshot(exact.raw, &sources).expect("authenticated exact cleanup source");
    let program = lower(pair_input(&syntax, &sources)).expect("sealed exact cleanup boundary");
    let module = program.modules().next().expect("one module");
    assert_eq!(module.functions().len(), 1);
    let function = module.functions().next().expect("one function");
    assert_eq!(function.blocks().len(), 1);
    assert_eq!(function.borrow_parameters().len(), 0);
    assert_eq!(function.places().len(), 1_069);
    let block = function.blocks().next().expect("one block");
    assert_eq!(block.terminator().edges().len(), 0);
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(instructions.len(), 1_581);
    assert_eq!(
        instructions.iter().filter(|instruction| instruction.result().is_some()).count(),
        558
    );
    assert!(
        instructions.len() < zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
    );
    assert!(function.places().len() < zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION);
    let begin = instructions
        .iter()
        .position(|i| i.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("shared begin");
    let end = instructions
        .iter()
        .position(|i| i.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("lexical end");
    assert_eq!(instructions.iter().filter(|i| i.borrow().is_some()).count(), 2);
    assert_eq!(instructions[begin].borrow_access(), Some(VerifiedBorrowAccess::Shared));
    assert_eq!(instructions[begin].borrow(), instructions[end].borrow());
    let root = instructions[begin].place_operands().next().expect("borrowed root");
    assert_eq!(instructions[end + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(instructions[end + 1].place_operands().collect::<Vec<_>>(), [root]);
    assert_eq!(end + 2, instructions.len());
    assert_eq!(block.terminator().value_operands().next(), instructions[end + 1].result());
    let clones = instructions
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind() == VerifiedInstructionKind::ClonePlace)
        .collect::<Vec<_>>();
    assert_eq!(clones.len(), 510);
    let mut cloned_locals = Vec::new();
    let mut temporary_owners = std::collections::BTreeSet::new();
    for (index, instruction) in clones {
        assert!(begin < index && index < end);
        assert_eq!(instruction.place_operands().collect::<Vec<_>>(), [root]);
        assert!(instruction.derived_drop_actions().any(|action| action.root() == root));
        let result = instruction.result().expect("distinct clone result");
        let owner = function
            .places()
            .find(|place| place.kind() == VerifiedPlaceKind::Temporary(result))
            .expect("unique clone temporary");
        assert_ne!(owner.id(), root);
        assert!(temporary_owners.insert(owner.id().index()));
        let initialize = instructions[index + 1];
        assert_eq!(initialize.kind(), VerifiedInstructionKind::InitializePlace);
        assert_eq!(initialize.value_operands().collect::<Vec<_>>(), [result]);
        cloned_locals.push(initialize.place_operands().next().expect("clone local"));
    }
    let drops = instructions
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind() == VerifiedInstructionKind::DropPlace)
        .map(|(index, instruction)| {
            assert!(begin < index && index < end);
            instruction.place_operands().next().expect("lexical drop")
        })
        .collect::<Vec<_>>();
    cloned_locals.reverse();
    assert_eq!(drops, cloned_locals);
    assert!(!drops.contains(&root));
    let plans = function.cleanup_plans().collect::<Vec<_>>();
    assert_eq!(plans.len(), 1_065);
    assert_eq!(plans[..44].iter().map(|plan| plan.actions().len()).sum::<usize>(), 514);
    for (index, pair) in plans[44..1_064].chunks_exact(2).enumerate() {
        assert_eq!(pair[0].actions().len(), index + 1);
        assert_eq!(pair[1].actions().len(), index + 2);
    }
    let return_cleanup = plans.last().expect("final return cleanup");
    assert_eq!(return_cleanup.id(), block.terminator().cleanup().expect("return cleanup identity"));
    assert_eq!(return_cleanup.actions().len(), 0);
    let remaining_actions = plans.iter().map(|plan| plan.actions().len()).sum::<usize>();
    assert_eq!(remaining_actions, 261_634);
    assert_eq!(remaining_actions + drops.len(), limit);

    let extra_sources = sources_for(&extra.source);
    let extra_syntax = verify_snapshot(extra.raw, &extra_sources)
        .expect("authenticated first-extra cleanup source");
    let reject = || {
        lower(pair_input(&extra_syntax, &extra_sources)).expect_err("first extra cleanup action")
    };
    let diagnostics = reject();
    let return_span = extra_sources.verify_span(extra.return_span).expect("final return span");
    assert_eq!(
        diagnostics,
        [zryna_diagnostics::Diagnostic::error_at(
            "ZRYNA-M3201",
            return_span,
            "derived cleanup actions exceed the per-function M3 limit of 262144",
            "reduce simultaneously live owned aggregates and String leaves",
        )]
    );
    assert_eq!(diagnostics, reject());
    let recovered =
        lower(pair_input(&syntax, &sources)).expect("exact case recovers after rejection");
    assert_eq!(cleanup_boundary_observation(&program), cleanup_boundary_observation(&recovered));
}

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
