use super::*;

#[test]
fn private_vec_string_constructor_consumes_elements_after_failure_cleanup() {
    let sources = sources_for(VEC_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(VEC_STRING_RESPONSE), &sources)
        .expect("source-faithful Vec<String> v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String>");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let construct = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecConstruct)
        .expect("VecConstruct");
    assert_eq!(
        construct.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next().is_some_and(|place| place.index() == 1)
    }));
}

#[test]
fn private_vec_i32_clone_preserves_source_and_returns_distinct_owner() {
    let (source, raw) = private_vec_clone_fixture("i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<i32> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<i32> clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone");
    let source_place = clone.place_operands().next().expect("clone source");
    let result = clone.result().expect("clone result");
    assert_eq!(source_place.index(), 1);
    assert_eq!(result.index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        block.terminator().value_operands().next().expect("returned clone").index(),
        result.index()
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1]
    );
    let abi = program.runtime_abi();
    assert_all_runtime_faults(
        abi,
        function,
        clone,
        LogicalOperation::VecAllocate,
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
    let fault = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("authenticated VecClone allocation failure");
    assert!(!fault.result_committed);
    assert_eq!(fault.uncommitted_result.expect("uncommitted clone result").index(), result.index());
    assert_eq!(fault.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
    assert_eq!(fault.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(), [1]);
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic replay");
    let replay_clone = replay
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("replayed VecClone");
    assert_eq!(
        (
            replay_clone.place_operands().next().expect("source").index(),
            replay_clone.result().expect("result").index(),
            replay_clone
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
        ),
        (source_place.index(), result.index(), vec![1])
    );
}

#[test]
fn private_vec_bool_clone_uses_the_same_copy_only_contract() {
    let (source, raw) = private_vec_clone_fixture("bool");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<bool> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<bool> clone");
    let clone = program
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone<bool>");
    assert_eq!(clone.place_operands().next().expect("source").index(), 1);
    assert_eq!(clone.result().expect("result").index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_string_clone_seals_allocation_and_prefix_failures() {
    let (source, raw) = private_vec_clone_fixture("String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> clone");
    let program = lower(pair_input(&syntax, &sources)).expect("private Vec<String> clone");
    let abi = program.runtime_abi();
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let clone = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("VecClone<String>");
    assert_eq!(clone.place_operands().next().expect("source").index(), 4);
    assert_eq!(clone.result().expect("result").index(), 4);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [4]
    );
    let element_actions = clone.vec_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        element_actions
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::kind)
            .collect::<Vec<_>>(),
        [VerifiedDropActionKind::VecInitializedPrefix, VerifiedDropActionKind::Place]
    );
    assert_eq!(
        element_actions.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
        [5, 4]
    );

    let allocation = owned_fault_trace(
        abi,
        function,
        clone,
        OwnedFaultInjection::Runtime {
            operation: LogicalOperation::VecAllocate,
            status: RuntimeStatus::Allocation,
        },
        0,
        1,
    )
    .expect("allocation phase");
    assert_eq!(
        allocation.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4]
    );
    assert_eq!(
        allocation.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
        [4]
    );
    assert!(allocation.prefix_owner.is_none());

    for (status, expected) in [
        (
            RuntimeStatus::Allocation,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::AllocationV1),
        ),
        (
            RuntimeStatus::Capacity,
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::CapacityV1),
        ),
        (RuntimeStatus::AbiViolation, OwnedFaultDisposition::HostFailure),
    ] {
        for completed_prefix in [0, 1, 2] {
            let injection =
                OwnedFaultInjection::VecCloneElement { status, source_length: 3, completed_prefix };
            let event_limit = usize::try_from(completed_prefix).expect("small prefix") + 1;
            let first = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
                .expect("element clone failure");
            let second = owned_fault_trace(abi, function, clone, injection, 0, event_limit)
                .expect("deterministic element failure");
            assert_eq!(first, second);
            assert_eq!(first.disposition, expected);
            assert_eq!(
                first.retained_roots.iter().map(|place| place.index()).collect::<Vec<_>>(),
                [4]
            );
            assert_eq!(
                first.reverse_cleanup.iter().map(|place| place.index()).collect::<Vec<_>>(),
                [4]
            );
            assert_eq!(first.prefix_owner.expect("prefix owner").index(), 5);
            assert_eq!(first.reverse_prefix, (0..completed_prefix).rev().collect::<Vec<_>>());
        }
    }
    let middle = OwnedFaultInjection::VecCloneElement {
        status: RuntimeStatus::Allocation,
        source_length: 3,
        completed_prefix: 2,
    };
    assert_eq!(
        owned_fault_trace(abi, function, clone, middle, 0, 2),
        Err(OwnedFaultOracleError::EventLimit)
    );
    assert_eq!(
        owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::VecCloneElement {
                status: RuntimeStatus::Allocation,
                source_length: MAX_VEC_ELEMENTS,
                completed_prefix: u64::MAX,
            },
            usize::MAX,
            usize::MAX,
        ),
        Err(OwnedFaultOracleError::InvalidVecClonePrefix)
    );
    for (source_length, completed_prefix) in [(3, 3), (MAX_VEC_ELEMENTS + 1, 0), (0, 0)] {
        assert_eq!(
            owned_fault_trace(
                abi,
                function,
                clone,
                OwnedFaultInjection::VecCloneElement {
                    status: RuntimeStatus::Allocation,
                    source_length,
                    completed_prefix,
                },
                0,
                usize::MAX,
            ),
            Err(OwnedFaultOracleError::InvalidVecClonePrefix)
        );
    }

    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic Vec<String> replay");
    let replay_clone = replay
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::VecClone)
        .expect("replayed clone");
    assert_eq!(
        replay_clone
            .vec_clone_element_failure_drop_actions()
            .map(|action| (action.kind(), action.root().index()))
            .collect::<Vec<_>>(),
        element_actions
            .iter()
            .map(|action| (action.kind(), action.root().index()))
            .collect::<Vec<_>>()
    );
}
