use super::*;

#[test]
fn private_vec_index_keeps_vector_for_fault_and_scalar_return_cleanup() {
    let sources = sources_for(VEC_INDEX_SOURCE);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let syntax = verify_snapshot(response_snapshot(&response), &sources)
        .expect("source-faithful Vec<i32> index v4");
    let program = lower(pair_input(&syntax, &sources)).expect("checked Vec<i32> index");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        function.parameters().len()
            + block.parameters().len()
            + block.instructions().filter(|instruction| instruction.result().is_some()).count(),
        5,
        "Vec construction emits three values and checked indexing emits index plus result",
    );
    let index = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecIndexCopy)
        .expect("VecIndexCopy");
    assert_eq!(
        index.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn owned_fault_oracle_covers_every_admitted_string_runtime_failure() {
    let allocation_capacity_host = [
        (
            RuntimeStatus::Allocation,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
        ),
        (
            RuntimeStatus::Capacity,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
        ),
        (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
    ];

    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources)
        .expect("source-faithful String literal");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let literal = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .expect("StringFromUtf8");
    assert_all_runtime_faults(
        abi,
        function,
        literal,
        LogicalOperation::StringFromUtf8Copy,
        &[
            (
                RuntimeStatus::Allocation,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
            ),
            (
                RuntimeStatus::Capacity,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
            ),
            (
                RuntimeStatus::Utf8,
                OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::Utf8V1),
            ),
            (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
        ],
    );

    let sources = sources_for(STRING_CLONE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CLONE_RESPONSE), &sources)
        .expect("source-faithful String clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private String clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("StringClone");
    assert_all_runtime_faults(
        abi,
        function,
        clone,
        LogicalOperation::StringClone,
        &allocation_capacity_host,
    );
    assert_eq!(
        owned_fault_trace(
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
        .expect("clone allocation trace")
        .reverse_cleanup
        .iter()
        .map(|place| place.index())
        .collect::<Vec<_>>(),
        [1]
    );

    let sources = sources_for(STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful String concat");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let concat = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringConcat)
        .expect("StringConcat");
    assert_all_runtime_faults(
        abi,
        function,
        concat,
        LogicalOperation::StringConcat,
        &allocation_capacity_host,
    );
    let trace = owned_fault_trace(
        abi,
        function,
        concat,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringConcat,
            status: RuntimeStatus::Capacity,
        },
        0,
        1,
    )
    .expect("concat capacity trace");
    assert_eq!(trace.block, 0);
    assert_eq!(trace.instruction, 4);
    assert_eq!(trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [3, 1]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn owned_fault_oracle_covers_vec_failures_bounds_and_nested_cleanup() {
    let (source, raw) = private_vec_nested_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec<String>");
    let program = lower(pair_input(&syntax, &sources)).expect("nested Vec<String>");
    let abi = program.runtime_abi();
    let function = program
        .verified_ir()
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    for (kind, operation) in [
        (VerifiedInstructionKind::VecConstruct, LogicalOperation::VecAllocate),
        (VerifiedInstructionKind::VecPush, LogicalOperation::VecReserve),
    ] {
        let instruction = instructions
            .iter()
            .copied()
            .find(|instruction| instruction.kind() == kind)
            .expect("Vec operation");
        assert_all_runtime_faults(
            abi,
            function,
            instruction,
            operation,
            &[
                (
                    RuntimeStatus::Allocation,
                    OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
                ),
                (
                    RuntimeStatus::Capacity,
                    OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
                ),
                (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
            ],
        );
    }
    let construct = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .expect("VecConstruct");
    let push = instructions
        .iter()
        .copied()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecPush)
        .expect("VecPush");
    let construct_trace = owned_fault_trace(
        abi,
        function,
        construct,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("nested construct failure");
    let push_trace = owned_fault_trace(
        abi,
        function,
        push,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecReserve,
            status: RuntimeStatus::Capacity,
        },
        0,
        1,
    )
    .expect("nested push failure");
    assert_eq!(
        construct_trace.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(
        construct_trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [2, 1, 0],
        "nested concat result and both read temporaries reverse-drop on construct failure"
    );
    assert_eq!(
        push_trace.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4, 7]
    );
    assert_eq!(
        push_trace.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [7, 6, 5, 4, 1, 0],
        "push argument temporaries, vector survivor, and earlier nested survivors reverse-drop"
    );

    let sources = sources_for(VEC_INDEX_SOURCE);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let syntax =
        verify_snapshot(response_snapshot(&response), &sources).expect("source-faithful Vec index");
    let program = lower(pair_input(&syntax, &sources)).expect("checked Vec index");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let index = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecIndexCopy)
        .expect("VecIndexCopy");
    let first = owned_fault_trace(abi, function, index, OwnedFaultInjection::Bounds, 0, 1)
        .expect("bounds trace");
    let second = owned_fault_trace(abi, function, index, OwnedFaultInjection::Bounds, 0, 1)
        .expect("deterministic bounds trace");
    assert_eq!(first, second);
    assert_eq!(
        first.disposition,
        OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::BoundsV1)
    );
    assert_eq!((first.block, first.instruction), (0, 5));
    assert_eq!(first.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
}

#[test]
fn owned_fault_oracle_is_bounded_and_fails_closed_on_mismatch() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources)
        .expect("source-faithful String literal");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let literal = function.blocks().next().expect("block").instructions().next().expect("literal");
    let valid = OwnedFaultInjection::Runtime {
        operation: LogicalOperation::StringFromUtf8Copy,
        status: RuntimeStatus::Utf8,
    };
    assert!(owned_fault_trace(abi, function, literal, valid, 0, 1).is_ok(), "exact event limit");
    assert_eq!(
        owned_fault_trace(abi, function, literal, valid, 1, 1),
        Err(OwnedFaultOracleError::EventLimit)
    );
    assert_eq!(
        owned_fault_trace(abi, function, literal, valid, usize::MAX, usize::MAX),
        Err(OwnedFaultOracleError::EventLimit)
    );
    for injection in [
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringFromUtf8Copy,
            status: RuntimeStatus::Ok,
        },
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringClone,
            status: RuntimeStatus::Allocation,
        },
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::StringFromUtf8Copy,
            status: RuntimeStatus::Refcount,
        },
        OwnedFaultInjection::Bounds,
    ] {
        assert!(owned_fault_trace(abi, function, literal, injection, 0, 1).is_err());
    }
}
