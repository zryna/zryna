use super::*;

const CONTRACT: &str = "runtime transition claim violates the frozen v1 state/result contract";

fn claim(
    operation: LogicalOperation,
    before: ControlState,
    status: RuntimeStatus,
    after: ControlState,
) -> TransitionClaim {
    TransitionClaim::Control { operation, before, status, bool_result: None, after }
}

fn rejected(input: TransitionClaim) {
    let first = validate_transition(input).expect_err("invalid count/status claim");
    assert_eq!(first.code(), "ZRYNA-R3002");
    assert_eq!(first.message(), CONTRACT);
    assert_eq!(first, validate_transition(input).expect_err("deterministic rejection"));
    validate_transition(claim(
        LogicalOperation::StrongClone,
        state(1, 1, false, true),
        RuntimeStatus::Ok,
        state(2, 1, false, true),
    ))
    .expect("valid transition after rejection");
}

#[test]
fn conformance_counts_all_increment_boundaries_preserve_exact_other_state() {
    // Synthetic ABI count claims: no assertion of billions of issued source handles.
    for operation in [
        LogicalOperation::StrongClone,
        LogicalOperation::WeakClone,
        LogicalOperation::WeakDowngrade,
        LogicalOperation::WeakUpgrade,
    ] {
        let strong =
            matches!(operation, LogicalOperation::StrongClone | LogicalOperation::WeakUpgrade);
        for count in [1, u32::MAX - 1, u32::MAX] {
            let before =
                if strong { state(count, 2, false, true) } else { state(1, count, false, true) };
            let (status, after) = if count == u32::MAX {
                (RuntimeStatus::Refcount, before)
            } else if strong {
                (RuntimeStatus::Ok, ControlState { strong_count: count + 1, ..before })
            } else {
                (RuntimeStatus::Ok, ControlState { weak_count: count + 1, ..before })
            };
            validate_transition(claim(operation, before, status, after))
                .expect("exact increment or atomic overflow");
            rejected(claim(operation, before, RuntimeStatus::Expired, before));
            rejected(claim(
                operation,
                before,
                status,
                ControlState { pending_last_strong: true, ..after },
            ));
            if count == u32::MAX {
                rejected(claim(
                    operation,
                    before,
                    status,
                    ControlState {
                        strong_count: 0,
                        weak_count: 0,
                        allocated: false,
                        payload_initialized: false,
                        pending_last_strong: false,
                    },
                ));
            } else {
                rejected(claim(operation, before, RuntimeStatus::Refcount, before));
            }
        }
    }
}

#[test]
fn conformance_counts_expired_weak_clone_is_not_upgrade_or_live_downgrade() {
    for weak in [1, u32::MAX - 1, u32::MAX] {
        let before = state(0, weak, false, false);
        validate_transition(claim(
            LogicalOperation::WeakUpgrade,
            before,
            RuntimeStatus::Expired,
            before,
        ))
        .expect("zero strong count expires without mutation");
        let status = if weak == u32::MAX { RuntimeStatus::Refcount } else { RuntimeStatus::Ok };
        let after =
            if weak == u32::MAX { before } else { ControlState { weak_count: weak + 1, ..before } };
        validate_transition(claim(LogicalOperation::WeakClone, before, status, after))
            .expect("explicit Weak clone remains legal after payload destruction");
        for operation in [
            LogicalOperation::StrongClone,
            LogicalOperation::WeakDowngrade,
            LogicalOperation::WeakUpgrade,
        ] {
            rejected(claim(operation, before, RuntimeStatus::Ok, before));
        }
        rejected(claim(LogicalOperation::WeakUpgrade, before, RuntimeStatus::Refcount, before));
    }
    let absent = state(0, 0, false, false);
    for operation in [
        LogicalOperation::StrongClone,
        LogicalOperation::WeakClone,
        LogicalOperation::WeakDowngrade,
    ] {
        rejected(claim(operation, absent, RuntimeStatus::Ok, absent));
    }
}
