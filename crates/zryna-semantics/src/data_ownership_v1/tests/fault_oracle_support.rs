use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnedFaultInjection {
    Runtime { operation: LogicalOperation, status: RuntimeStatus },
    VecCloneElement { status: RuntimeStatus, source_length: u64, completed_prefix: u64 },
    AggregateCloneElement { status: RuntimeStatus, completed_prefix: u64 },
    Bounds,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnedFaultDisposition {
    ControlledTrap(VerifiedTrapIdentity),
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedFaultTrace {
    kind: VerifiedInstructionKind,
    span: FaultSpan,
    pub(super) block: u32,
    pub(super) instruction: u32,
    pub(super) disposition: OwnedFaultDisposition,
    pub(super) result_committed: bool,
    pub(super) uncommitted_result: Option<FaultValueIdentity>,
    pub(super) retained_roots: Vec<FaultPlaceIdentity>,
    pub(super) reverse_cleanup: Vec<FaultPlaceIdentity>,
    pub(super) prefix_owner: Option<FaultPlaceIdentity>,
    pub(super) reverse_prefix: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnedFaultOracleError {
    StatusMismatch,
    SuccessStatus,
    MissingPrepareCleanup,
    AtomicityMismatch,
    EventLimit,
    InvalidVecClonePrefix,
    InvalidAggregateClonePrefix,
}

fn runtime_operation(kind: VerifiedInstructionKind) -> Option<LogicalOperation> {
    match kind {
        VerifiedInstructionKind::StringFromUtf8 => Some(LogicalOperation::StringFromUtf8Copy),
        VerifiedInstructionKind::StringClone => Some(LogicalOperation::StringClone),
        VerifiedInstructionKind::StringConcat => Some(LogicalOperation::StringConcat),
        VerifiedInstructionKind::VecClone | VerifiedInstructionKind::VecConstruct => {
            Some(LogicalOperation::VecAllocate)
        }
        VerifiedInstructionKind::VecPush => Some(LogicalOperation::VecReserve),
        _ => None,
    }
}

fn runtime_fault_disposition(
    abi: &VerifiedOwnershipRuntimeAbi,
    status: RuntimeStatus,
) -> Option<OwnedFaultDisposition> {
    let declaration =
        abi.status_declarations().find(|declaration| declaration.status() == status)?;
    match (declaration.disposition(), declaration.trap_identity()) {
        (VerifiedStatusDisposition::ControlledTrap, Some(trap)) => {
            let identity = match trap {
                VerifiedStatusTrapIdentity::AllocationV1 => VerifiedTrapIdentity::AllocationV1,
                VerifiedStatusTrapIdentity::CapacityV1 => VerifiedTrapIdentity::CapacityV1,
                VerifiedStatusTrapIdentity::RefcountV1 => VerifiedTrapIdentity::RefcountV1,
                VerifiedStatusTrapIdentity::Utf8V1 => VerifiedTrapIdentity::Utf8V1,
            };
            Some(OwnedFaultDisposition::ControlledTrap(identity))
        }
        (VerifiedStatusDisposition::HostFailure, None) => Some(OwnedFaultDisposition::HostFailure),
        _ => None,
    }
}

fn owned_fault_root(
    function: VerifiedFunction<'_>,
    mut place: FaultPlaceIdentity,
) -> Result<FaultPlaceIdentity, OwnedFaultOracleError> {
    let limit = function.places().count();
    for _ in 0..=limit {
        let verified = function
            .places()
            .find(|candidate| candidate.id() == place)
            .ok_or(OwnedFaultOracleError::AtomicityMismatch)?;
        place = match verified.kind() {
            VerifiedPlaceKind::StructField { base, .. }
            | VerifiedPlaceKind::EnumPayload { base, .. }
            | VerifiedPlaceKind::FixedArrayConstant { base, .. } => base,
            VerifiedPlaceKind::Parameter(_)
            | VerifiedPlaceKind::Local(_)
            | VerifiedPlaceKind::Temporary(_) => return Ok(place),
        };
    }
    Err(OwnedFaultOracleError::AtomicityMismatch)
}

#[allow(clippy::too_many_lines)]
pub(super) fn owned_fault_trace(
    abi: &VerifiedOwnershipRuntimeAbi,
    function: VerifiedFunction<'_>,
    instruction: FaultVerifiedInstruction<'_>,
    injection: OwnedFaultInjection,
    retained_events: usize,
    event_limit: usize,
) -> Result<OwnedFaultTrace, OwnedFaultOracleError> {
    let prefix_events = match injection {
        OwnedFaultInjection::VecCloneElement { source_length, completed_prefix, .. } => {
            if source_length > MAX_VEC_ELEMENTS || completed_prefix >= source_length {
                return Err(OwnedFaultOracleError::InvalidVecClonePrefix);
            }
            usize::try_from(completed_prefix).map_err(|_| OwnedFaultOracleError::EventLimit)?
        }
        OwnedFaultInjection::AggregateCloneElement { completed_prefix, .. } => {
            let leaf_count = instruction
                .aggregate_clone_fallible_leaf_count()
                .ok_or(OwnedFaultOracleError::StatusMismatch)?;
            if completed_prefix >= leaf_count {
                return Err(OwnedFaultOracleError::InvalidAggregateClonePrefix);
            }
            usize::try_from(completed_prefix).map_err(|_| OwnedFaultOracleError::EventLimit)?
        }
        _ => 0,
    };
    let new_events = prefix_events.checked_add(1).ok_or(OwnedFaultOracleError::EventLimit)?;
    if retained_events.checked_add(new_events).is_none_or(|total| total > event_limit) {
        return Err(OwnedFaultOracleError::EventLimit);
    }
    let kind = instruction.kind();
    let disposition = match injection {
        OwnedFaultInjection::Bounds if kind == VerifiedInstructionKind::VecIndexCopy => {
            OwnedFaultDisposition::ControlledTrap(VerifiedTrapIdentity::BoundsV1)
        }
        OwnedFaultInjection::Bounds => return Err(OwnedFaultOracleError::StatusMismatch),
        OwnedFaultInjection::Runtime { status: RuntimeStatus::Ok, .. }
        | OwnedFaultInjection::VecCloneElement { status: RuntimeStatus::Ok, .. }
        | OwnedFaultInjection::AggregateCloneElement { status: RuntimeStatus::Ok, .. } => {
            return Err(OwnedFaultOracleError::SuccessStatus);
        }
        OwnedFaultInjection::Runtime { operation, status } => {
            let Some(expected) = runtime_operation(kind) else {
                return Err(OwnedFaultOracleError::StatusMismatch);
            };
            if operation != expected || !operation_accepts_status(operation, status) {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(operation, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
        OwnedFaultInjection::VecCloneElement { status, .. } => {
            if kind != VerifiedInstructionKind::VecClone
                || !operation_accepts_status(LogicalOperation::StringClone, status)
            {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(LogicalOperation::StringClone, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
        OwnedFaultInjection::AggregateCloneElement { status, .. } => {
            if kind != VerifiedInstructionKind::ClonePlace
                || !operation_accepts_status(LogicalOperation::StringClone, status)
            {
                return Err(OwnedFaultOracleError::StatusMismatch);
            }
            validate_failure_atomic_transition(LogicalOperation::StringClone, status, true, true)
                .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)?;
            runtime_fault_disposition(abi, status).ok_or(OwnedFaultOracleError::StatusMismatch)?
        }
    };
    let vec_element_failure = matches!(injection, OwnedFaultInjection::VecCloneElement { .. });
    let aggregate_element_failure =
        matches!(injection, OwnedFaultInjection::AggregateCloneElement { .. });
    let element_failure = vec_element_failure || aggregate_element_failure;
    let cleanup = if vec_element_failure {
        instruction
            .vec_clone_element_cleanup()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    } else if aggregate_element_failure {
        instruction
            .aggregate_clone_element_cleanup()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    } else {
        instruction.cleanup().ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?
    };
    let plan = function
        .cleanup_plans()
        .find(|plan| plan.id() == cleanup)
        .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?;
    let site = plan.site();
    let expected_role = if vec_element_failure {
        VerifiedCleanupRole::VecCloneElementFailure
    } else if aggregate_element_failure {
        VerifiedCleanupRole::AggregateCloneElementFailure
    } else {
        VerifiedCleanupRole::PrepareFailure
    };
    if site.role() != expected_role {
        return Err(OwnedFaultOracleError::MissingPrepareCleanup);
    }
    let actions = if vec_element_failure {
        instruction.vec_clone_element_failure_drop_actions().collect::<Vec<_>>()
    } else if aggregate_element_failure {
        instruction.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>()
    } else {
        instruction.derived_drop_actions().collect::<Vec<_>>()
    };
    let (prefix_owner, reverse_cleanup) = if element_failure {
        let Some((prefix, remaining)) = actions.split_first() else {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        };
        let expected_prefix = if vec_element_failure {
            VerifiedDropActionKind::VecInitializedPrefix
        } else {
            VerifiedDropActionKind::AggregateInitializedPrefix
        };
        if prefix.kind() != expected_prefix
            || remaining.iter().any(|action| action.kind() != VerifiedDropActionKind::Place)
        {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
        (
            Some(prefix.root()),
            remaining
                .iter()
                .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
                .collect::<Vec<_>>(),
        )
    } else {
        if actions.iter().any(|action| action.kind() != VerifiedDropActionKind::Place) {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
        (
            None,
            actions
                .iter()
                .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
                .collect::<Vec<_>>(),
        )
    };
    let mut retained_roots = Vec::new();
    for place in instruction.place_operands() {
        let owner = owned_fault_root(function, place)?;
        if !retained_roots.contains(&owner) {
            retained_roots.push(owner);
        }
    }
    for value in instruction.value_operands() {
        let candidates = function
            .places()
            .filter(|place| place.kind() == VerifiedPlaceKind::Temporary(value))
            .collect::<Vec<_>>();
        match candidates.as_slice() {
            [] => {}
            [candidate] if candidate.is_copy() => {}
            [candidate] => {
                let owner = candidate.id();
                if !retained_roots.contains(&owner) {
                    retained_roots.push(owner);
                }
            }
            _ => return Err(OwnedFaultOracleError::AtomicityMismatch),
        }
    }
    if retained_roots.iter().any(|owner| !reverse_cleanup.contains(owner)) {
        return Err(OwnedFaultOracleError::AtomicityMismatch);
    }
    if let Some(result) = instruction.result()
        && function.places().any(|place| {
            place.kind() == VerifiedPlaceKind::Temporary(result)
                && reverse_cleanup.contains(&place.id())
        })
    {
        return Err(OwnedFaultOracleError::AtomicityMismatch);
    }
    if let (Some(result), Some(prefix)) = (instruction.result(), prefix_owner) {
        let matches_result = function.places().any(|place| {
            place.kind() == VerifiedPlaceKind::Temporary(result) && place.id() == prefix
        });
        if !matches_result {
            return Err(OwnedFaultOracleError::AtomicityMismatch);
        }
    }
    let reverse_prefix = match injection {
        OwnedFaultInjection::VecCloneElement { completed_prefix, .. }
        | OwnedFaultInjection::AggregateCloneElement { completed_prefix, .. } => {
            (0..completed_prefix).rev().collect()
        }
        _ => Vec::new(),
    };
    Ok(OwnedFaultTrace {
        kind,
        span: instruction.span(),
        block: site.block().index(),
        instruction: site
            .instruction_index()
            .ok_or(OwnedFaultOracleError::MissingPrepareCleanup)?,
        disposition,
        result_committed: false,
        uncommitted_result: instruction.result(),
        retained_roots,
        reverse_cleanup,
        prefix_owner,
        reverse_prefix,
    })
}

pub(super) fn assert_all_runtime_faults(
    abi: &VerifiedOwnershipRuntimeAbi,
    function: VerifiedFunction<'_>,
    instruction: FaultVerifiedInstruction<'_>,
    operation: LogicalOperation,
    expected: &[(RuntimeStatus, OwnedFaultDisposition)],
) {
    let all = [
        RuntimeStatus::Allocation,
        RuntimeStatus::Capacity,
        RuntimeStatus::Refcount,
        RuntimeStatus::Utf8,
        RuntimeStatus::Expired,
        RuntimeStatus::AbiViolation,
    ];
    let admitted = all
        .into_iter()
        .filter(|status| operation_accepts_status(operation, *status))
        .collect::<Vec<_>>();
    assert_eq!(admitted, expected.iter().map(|(status, _)| *status).collect::<Vec<_>>());
    for &(status, expected_disposition) in expected {
        let injection = OwnedFaultInjection::Runtime { operation, status };
        let first = owned_fault_trace(abi, function, instruction, injection, 0, 1)
            .expect("admitted runtime fault");
        let second = owned_fault_trace(abi, function, instruction, injection, 0, 1)
            .expect("deterministic admitted runtime fault");
        assert_eq!(first, second);
        assert_eq!(first.kind, instruction.kind());
        assert_eq!(first.span, instruction.span());
        assert_eq!(first.disposition, expected_disposition);
        assert!(!first.result_committed);
        assert_eq!(first.uncommitted_result, instruction.result());
        assert!(
            first.retained_roots.iter().all(|owner| first.reverse_cleanup.contains(owner)),
            "every precommit operand owner remains cleanup-authoritative"
        );
    }
}
