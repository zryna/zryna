use super::*;

pub(super) fn handle_clone_operations(
    instruction: FaultVerifiedInstruction<'_>,
) -> Option<Vec<LogicalOperation>> {
    let clone = instruction.handle_aware_clone()?;
    Some(
        clone
            .frontier()
            .nodes()
            .filter_map(|node| match node.kind() {
                VerifiedHandleCloneRecipeKind::StringClone => Some(LogicalOperation::StringClone),
                VerifiedHandleCloneRecipeKind::VecEach { .. } => {
                    Some(LogicalOperation::VecAllocate)
                }
                VerifiedHandleCloneRecipeKind::SharedCountClone => {
                    Some(LogicalOperation::StrongClone)
                }
                VerifiedHandleCloneRecipeKind::WeakCountClone => Some(LogicalOperation::WeakClone),
                VerifiedHandleCloneRecipeKind::Copy
                | VerifiedHandleCloneRecipeKind::Struct(_)
                | VerifiedHandleCloneRecipeKind::Enum(_)
                | VerifiedHandleCloneRecipeKind::FixedArray { .. } => None,
            })
            .collect(),
    )
}

pub(super) fn validate_handle_failure(
    operation: LogicalOperation,
    status: RuntimeStatus,
) -> Result<(), OwnedFaultOracleError> {
    if !operation_accepts_status(operation, status) {
        return Err(OwnedFaultOracleError::StatusMismatch);
    }
    if matches!(
        operation,
        LogicalOperation::StrongClone
            | LogicalOperation::WeakDowngrade
            | LogicalOperation::WeakClone
    ) {
        let mut before = ControlState {
            strong_count: 1,
            weak_count: 1,
            pending_last_strong: false,
            payload_initialized: true,
            allocated: true,
        };
        if status == RuntimeStatus::Refcount {
            if operation == LogicalOperation::StrongClone {
                before.strong_count = u32::MAX;
            } else {
                before.weak_count = u32::MAX;
            }
        }
        validate_transition(TransitionClaim::Control {
            operation,
            before,
            status,
            bool_result: None,
            after: before,
        })
        .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)
    } else {
        validate_failure_atomic_transition(operation, status, true, true)
            .map_err(|_| OwnedFaultOracleError::AtomicityMismatch)
    }
}
