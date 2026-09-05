use super::generic_vec_fixture::shared_weak_fixture::{composition_fixture, fixture};
use super::*;

fn disposition(status: RuntimeStatus) -> OwnedFaultDisposition {
    match status {
        RuntimeStatus::Allocation => {
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1)
        }
        RuntimeStatus::Capacity => {
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1)
        }
        RuntimeStatus::Refcount => {
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::RefcountV1)
        }
        RuntimeStatus::AbiViolation => OwnedFaultDisposition::HostFailure,
        _ => panic!("status is not a handle fault"),
    }
}

fn operation_fault(operation: LogicalOperation) -> RuntimeStatus {
    match operation {
        LogicalOperation::StrongClone
        | LogicalOperation::WeakDowngrade
        | LogicalOperation::WeakClone => RuntimeStatus::Refcount,
        LogicalOperation::StringClone | LogicalOperation::VecAllocate => RuntimeStatus::Allocation,
        _ => panic!("not one structural handle clone step"),
    }
}

fn recipe_operations(instruction: FaultVerifiedInstruction<'_>) -> Vec<LogicalOperation> {
    instruction
        .handle_aware_clone()
        .expect("handle-aware clone")
        .frontier()
        .nodes()
        .filter_map(|node| match node.kind() {
            VerifiedHandleCloneRecipeKind::StringClone => Some(LogicalOperation::StringClone),
            VerifiedHandleCloneRecipeKind::VecEach { .. } => Some(LogicalOperation::VecAllocate),
            VerifiedHandleCloneRecipeKind::SharedCountClone => Some(LogicalOperation::StrongClone),
            VerifiedHandleCloneRecipeKind::WeakCountClone => Some(LogicalOperation::WeakClone),
            VerifiedHandleCloneRecipeKind::Copy
            | VerifiedHandleCloneRecipeKind::Struct(_)
            | VerifiedHandleCloneRecipeKind::Enum(_)
            | VerifiedHandleCloneRecipeKind::FixedArray { .. } => None,
        })
        .collect()
}

#[test]
fn direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup() {
    let (source, raw) = fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated direct handle source");
    let program = lower(pair_input(&syntax, &sources)).expect("verified direct handle IR");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    for (kind, operation, statuses) in [
        (
            VerifiedInstructionKind::SharedConstruct,
            LogicalOperation::Allocate,
            &[RuntimeStatus::Allocation, RuntimeStatus::Capacity, RuntimeStatus::AbiViolation][..],
        ),
        (
            VerifiedInstructionKind::SharedClone,
            LogicalOperation::StrongClone,
            &[RuntimeStatus::Refcount, RuntimeStatus::AbiViolation][..],
        ),
        (
            VerifiedInstructionKind::WeakDowngrade,
            LogicalOperation::WeakDowngrade,
            &[RuntimeStatus::Refcount, RuntimeStatus::AbiViolation][..],
        ),
        (
            VerifiedInstructionKind::WeakClone,
            LogicalOperation::WeakClone,
            &[RuntimeStatus::Refcount, RuntimeStatus::AbiViolation][..],
        ),
    ] {
        let instruction = instructions
            .iter()
            .copied()
            .find(|instruction| instruction.kind() == kind)
            .expect("emitted handle operation");
        let expected = statuses
            .iter()
            .copied()
            .map(|status| (status, disposition(status)))
            .collect::<Vec<_>>();
        assert_all_runtime_faults(abi, function, instruction, operation, &expected);
        let trace = owned_fault_trace(
            abi,
            function,
            instruction,
            OwnedFaultInjection::Runtime { operation, status: statuses[0] },
            0,
            1,
        )
        .expect("bound handle fault");
        assert_eq!(
            (trace.operation, trace.status, trace.fault_ordinal),
            (Some(operation), Some(statuses[0]), Some(0))
        );
        assert!(!trace.result_committed);
        assert!(trace.uncommitted_result.is_some());
        assert!(trace.prefix_owner.is_none());
        assert_eq!(trace.retained_roots.len(), 1, "one exact source owner is retained");
        assert!(trace.retained_roots.iter().all(|root| trace.reverse_cleanup.contains(root)));
    }
    let replay = lower(pair_input(&syntax, &sources)).expect("valid recovery replay");
    assert_eq!(
        replay
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
            .filter(|instruction| matches!(
                instruction.kind(),
                VerifiedInstructionKind::SharedConstruct
                    | VerifiedInstructionKind::SharedClone
                    | VerifiedInstructionKind::WeakDowngrade
                    | VerifiedInstructionKind::WeakClone
            ))
            .count(),
        4
    );
}

#[test]
fn structural_handle_fault_ordinals_bind_prefix_cleanup_and_source_retention() {
    let (source, raw) = composition_fixture::clone_rejection_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated structural handle source");
    let program = lower(pair_input(&syntax, &sources)).expect("verified structural handle IR");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().nth(1).expect("caller");
    let clones = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .filter(|instruction| instruction.handle_aware_clone().is_some())
        .collect::<Vec<_>>();
    assert_eq!(clones.len(), 6);
    for clone_instruction in clones {
        let clone = clone_instruction.handle_aware_clone().expect("clone contract");
        let operations = recipe_operations(clone_instruction);
        assert!(!operations.is_empty());
        for (ordinal, operation) in operations.iter().copied().enumerate() {
            let status = operation_fault(operation);
            let injection = OwnedFaultInjection::HandleCloneStep {
                operation,
                status,
                fault_ordinal: ordinal as u32,
            };
            let first =
                owned_fault_trace(abi, function, clone_instruction, injection, 0, ordinal + 1)
                    .expect("symbolic clone-step fault");
            let second =
                owned_fault_trace(abi, function, clone_instruction, injection, 0, ordinal + 1)
                    .expect("deterministic clone-step replay");
            assert_eq!(first, second);
            assert_eq!(
                (first.operation, first.status, first.fault_ordinal),
                (Some(operation), Some(status), Some(ordinal as u32))
            );
            assert_eq!(first.disposition, disposition(status));
            assert_eq!(first.prefix_owner, Some(clone.destination()));
            assert_eq!(first.retained_roots, [clone.source_root()]);
            assert!(first.reverse_cleanup.contains(&clone.source_root()));
            assert_eq!(first.reverse_prefix, (0..ordinal as u64).rev().collect::<Vec<_>>());
            assert!(!first.result_committed);
            assert_eq!(first.uncommitted_result, Some(clone.result()));
            assert_eq!(
                owned_fault_trace(abi, function, clone_instruction, injection, 0, ordinal,),
                Err(OwnedFaultOracleError::EventLimit)
            );
        }
        let invalid = OwnedFaultInjection::HandleCloneStep {
            operation: operations[0],
            status: operation_fault(operations[0]),
            fault_ordinal: operations.len() as u32,
        };
        assert_eq!(
            owned_fault_trace(abi, function, clone_instruction, invalid, 0, operations.len() + 1),
            Err(OwnedFaultOracleError::InvalidHandleCloneOrdinal)
        );
        let wrong_status = if matches!(
            operations[0],
            LogicalOperation::StrongClone
                | LogicalOperation::WeakDowngrade
                | LogicalOperation::WeakClone
        ) {
            RuntimeStatus::Allocation
        } else {
            RuntimeStatus::Refcount
        };
        assert_eq!(
            owned_fault_trace(
                abi,
                function,
                clone_instruction,
                OwnedFaultInjection::HandleCloneStep {
                    operation: operations[0],
                    status: wrong_status,
                    fault_ordinal: 0,
                },
                0,
                1,
            ),
            Err(OwnedFaultOracleError::StatusMismatch)
        );
    }
    let recovery = lower(pair_input(&syntax, &sources)).expect("valid compile after fault replay");
    assert_eq!(recovery.verified_ir().identity(), program.verified_ir().identity());
}
