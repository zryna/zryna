use super::*;

fn model<'a>(abi: &'a VerifiedOwnershipRuntimeAbi, layouts: &'a VerifiedLayouts) -> Model<'a> {
    Model {
        abi,
        layouts,
        controls: vec![],
        owners: vec![],
        intervals: BTreeMap::new(),
        pending: vec![],
        allocations: 0,
        nodes: 0,
    }
}

#[test]
fn control_model_internal_resource_boundaries_publish_nothing_on_rejection() {
    // Private counter controls exercise real replay preflights, not million-allocation executions.
    let (abi, layouts, _) = authorities();
    let event = seed(&abi, &layouts);
    for allocations in [MAX_ALLOCATION_OPERATIONS - 1, MAX_ALLOCATION_OPERATIONS, u64::MAX] {
        let mut state = model(&abi, &layouts);
        state.allocations = allocations;
        let outcome = state.replay(&event);
        if allocations == MAX_ALLOCATION_OPERATIONS - 1 {
            outcome.expect("exact allocation count publishes");
            assert_eq!(state.owners.len(), 1);
        } else {
            let error = outcome.expect_err("first extra or arithmetic overflow");
            assert_eq!(error.code(), "ZRYNA-R3201");
            assert_eq!(error.message(), "control trace exceeds the checked invocation budget");
            assert!(
                state.owners.is_empty() && state.controls.is_empty() && state.intervals.is_empty()
            );
        }
        let mut recovery = model(&abi, &layouts);
        recovery.replay(&event).expect("valid replay after rejected invocation");
        assert_eq!(recovery.owners.len(), 1);
    }
    for nodes in [MAX_STATUS_TRANSITIONS - 1, MAX_STATUS_TRANSITIONS, u64::MAX] {
        let mut state = model(&abi, &layouts);
        state.nodes = nodes;
        let outcome = state.replay(&event);
        if nodes == MAX_STATUS_TRANSITIONS - 1 {
            outcome.expect("exact payload-node count publishes");
            assert_eq!(state.nodes, MAX_STATUS_TRANSITIONS);
        } else {
            assert_eq!(outcome.expect_err("node first extra or overflow").code(), "ZRYNA-R3201");
            assert!(
                state.owners.is_empty() && state.controls.is_empty() && state.intervals.is_empty()
            );
            assert_eq!(state.allocations, 0, "node preflight precedes allocation");
        }
    }
    let mut state = model(&abi, &layouts);
    state.allocations = MAX_ALLOCATION_OPERATIONS;
    let mut capacity = event.clone();
    if let ControlEventKind::Construct { status, base, control, owner, .. } = &mut capacity {
        *status = RuntimeStatus::Capacity;
        *base = 0;
        *control = None;
        *owner = None;
    }
    state.replay(&capacity).expect("capacity disposition is distinct from allocation failure");
    assert!(state.owners.is_empty() && state.controls.is_empty());
}

#[test]
fn control_model_synthetic_refcount_failure_cannot_issue_an_owner() {
    // Synthetic saturated count proofs, deliberately not authenticated billion-owner traces.
    let (abi, layouts, _) = authorities();
    for operation in [
        LogicalOperation::StrongClone,
        LogicalOperation::WeakUpgrade,
        LogicalOperation::WeakDowngrade,
        LogicalOperation::WeakClone,
    ] {
        for result in [None, Some(1)] {
            let mut replay = model(&abi, &layouts);
            replay.replay(&seed(&abi, &layouts)).expect("issued source identity");
            let weak =
                matches!(operation, LogicalOperation::WeakUpgrade | LogicalOperation::WeakClone);
            replay.owners[0].weak = weak;
            let strong_increment =
                matches!(operation, LogicalOperation::StrongClone | LogicalOperation::WeakUpgrade);
            let before = if strong_increment {
                state(u32::MAX, 2, false, true)
            } else {
                state(1, u32::MAX, false, true)
            };
            replay.controls[0].state = before;
            let event = ControlEventKind::Handle {
                operation,
                owner: 0,
                status: RuntimeStatus::Refcount,
                result,
                boolean: None,
                after: before,
            };
            let outcome = replay.replay(&event);
            if result.is_none() {
                outcome.expect("refcount failure retains exact owner set");
            } else {
                assert_eq!(
                    outcome.expect_err("forged result").message(),
                    "only successful clone or upgrade may issue one fresh owner"
                );
            }
            assert_eq!(replay.owners.len(), 1);
            assert_eq!(replay.owners[0].location, Location::External);
            assert_eq!(replay.controls[0].state, before);
        }
    }
}
