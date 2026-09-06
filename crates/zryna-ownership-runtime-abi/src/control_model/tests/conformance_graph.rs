use super::*;

fn observers(abi: &VerifiedOwnershipRuntimeAbi, layouts: &VerifiedLayouts) -> ControlTrace {
    let shared = layouts
        .types()
        .find(|record| {
            record.category() == TypeCategory::Shared
                && record.referenced_type() == Some(ty(layouts, TypeCategory::String))
        })
        .expect("Shared String")
        .id();
    trace(
        abi,
        layouts,
        vec![
            seed(abi, layouts),
            handle(LogicalOperation::StrongClone, 0, Some(1), None, state(2, 1, false, true)),
            handle(LogicalOperation::WeakDowngrade, 0, Some(2), None, state(2, 2, false, true)),
            construct(
                abi,
                layouts,
                vec![PayloadNode { ty: shared, kind: PayloadKind::Handle(0) }],
                1,
                3,
                2048,
            ),
            construct(
                abi,
                layouts,
                vec![PayloadNode { ty: shared, kind: PayloadKind::Handle(1) }],
                2,
                4,
                3072,
            ),
            handle(
                LogicalOperation::StrongReleaseBegin,
                3,
                None,
                Some(true),
                state(0, 1, true, true),
            ),
            handle(
                LogicalOperation::StrongReleaseBegin,
                0,
                None,
                Some(false),
                state(1, 2, false, true),
            ),
            ControlEventKind::Finish { control: 1, after: state(0, 0, false, false) },
            handle(
                LogicalOperation::StrongReleaseBegin,
                4,
                None,
                Some(true),
                state(0, 1, true, true),
            ),
            handle(
                LogicalOperation::StrongReleaseBegin,
                1,
                None,
                Some(true),
                state(0, 2, true, true),
            ),
            ControlEventKind::DropPayloadNode { control: 0, node: 0 },
            ControlEventKind::Finish { control: 0, after: state(0, 1, false, false) },
            ControlEventKind::Finish { control: 2, after: state(0, 0, false, false) },
            ControlEventKind::Handle {
                operation: LogicalOperation::WeakUpgrade,
                owner: 2,
                result: None,
                boolean: None,
                status: RuntimeStatus::Expired,
                after: state(0, 1, false, false),
            },
            handle(LogicalOperation::WeakRelease, 2, None, Some(true), state(0, 0, false, false)),
        ],
    )
}

fn exact_rejection(
    abi: &VerifiedOwnershipRuntimeAbi,
    layouts: &VerifiedLayouts,
    valid: &ControlTrace,
    mut bad: ControlTrace,
    code: &str,
    message: &str,
) {
    for (site, event) in bad.events.iter_mut().enumerate() {
        event.site = site as u64;
    }
    let pristine =
        verify_control_trace(abi, layouts, 17, &[], valid).expect("independent valid control");
    let first = verify_control_trace(abi, layouts, 17, &[], &bad).expect_err("hostile authority");
    assert_eq!((first.code(), first.message()), (code, message));
    assert_eq!(
        first,
        verify_control_trace(abi, layouts, 17, &[], &bad).expect_err("same rejection")
    );
    assert_eq!(
        verify_control_trace(abi, layouts, 17, &[], valid).expect("pristine recovery"),
        pristine
    );
}

#[test]
fn conformance_graph_distinct_cloned_edges_and_weak_observer_release_completely() {
    let (abi, linear, linux) = authorities();
    for layouts in [&linear, &linux] {
        let input = observers(&abi, layouts);
        let proof = verify_control_trace(&abi, layouts, 17, &[], &input)
            .expect("lawful shared target and observer");
        assert_eq!(proof.live_owners(), 0);
        assert_eq!(proof.states().collect::<Vec<_>>(), vec![state(0, 0, false, false); 3]);
        // The observer survives both strong owners and the complete nested payload release.
        let prefix = ControlTrace { events: input.events[..13].to_vec(), ..input.clone() };
        let expected = [ExpectedOwner { owner: 2, control: 0, weak: true }];
        let expired = verify_control_trace(&abi, layouts, 17, &expected, &prefix)
            .expect("only Weak survives");
        assert_eq!(expired.states().next(), Some(state(0, 1, false, false)));
        assert_eq!(expired.live_owners(), 1);
    }
}

#[test]
fn conformance_graph_forged_future_cycles_and_pending_interposition_reject_exactly() {
    let (abi, linear, linux) = authorities();
    for layouts in [&linear, &linux] {
        let valid = observers(&abi, layouts);
        for owner in [3, 4] {
            let mut bad = valid.clone();
            let ControlEventKind::Construct { nodes, .. } = &mut bad.events[3].kind else {
                panic!("construction")
            };
            nodes[0].kind = PayloadKind::Handle(owner);
            exact_rejection(
                &abi,
                layouts,
                &valid,
                bad,
                "ZRYNA-R3006",
                "payload handle was never issued",
            );
        }
        for owner in [3, 4] {
            let mut bad = valid.clone();
            bad.events[3].kind = construct(
                &abi,
                layouts,
                vec![PayloadNode {
                    ty: ty(layouts, TypeCategory::Weak),
                    kind: PayloadKind::Handle(owner),
                }],
                1,
                3,
                2048,
            );
            exact_rejection(
                &abi,
                layouts,
                &valid,
                bad,
                "ZRYNA-R3006",
                "payload handle was never issued",
            );
        }
        let mut duplicate = valid.clone();
        let ControlEventKind::Construct { nodes, .. } = &mut duplicate.events[4].kind else {
            panic!("construction")
        };
        nodes[0].kind = PayloadKind::Handle(0);
        exact_rejection(
            &abi,
            layouts,
            &valid,
            duplicate,
            "ZRYNA-R3006",
            "payload handle is foreign, moved, duplicated or has wrong referent",
        );
        for event in [
            handle(LogicalOperation::WeakRelease, 2, None, Some(false), state(2, 1, false, true)),
            handle(LogicalOperation::StrongClone, 1, Some(5), None, state(3, 2, false, true)),
        ] {
            let mut bad = valid.clone();
            bad.events.insert(6, ControlEvent { site: 6, kind: event });
            exact_rejection(
                &abi,
                layouts,
                &valid,
                bad,
                "ZRYNA-R3006",
                "payload cleanup cannot invoke unrelated or retained handle operations",
            );
        }
        let mut early = valid.clone();
        early.events.swap(10, 11);
        exact_rejection(
            &abi,
            layouts,
            &valid,
            early,
            "ZRYNA-R3006",
            "last-release finish precedes complete payload destruction",
        );
    }
}

#[test]
fn conformance_status_corruption_is_not_expiration_or_a_language_trap() {
    let (abi, linear, linux) = authorities();
    for layouts in [&linear, &linux] {
        let valid = observers(&abi, layouts);
        for (status, result, code, message) in [
            (
                RuntimeStatus::Expired,
                Some(5),
                "ZRYNA-R3006",
                "only successful clone or upgrade may issue one fresh owner",
            ),
            (
                RuntimeStatus::Refcount,
                None,
                "ZRYNA-R3002",
                "runtime transition claim violates the frozen v1 state/result contract",
            ),
            (
                RuntimeStatus::AbiViolation,
                None,
                "ZRYNA-R3006",
                "operation reenters a pending or deallocated control",
            ),
        ] {
            let mut bad = valid.clone();
            let ControlEventKind::Handle { status: actual, result: output, .. } =
                &mut bad.events[13].kind
            else {
                panic!("upgrade")
            };
            *actual = status;
            *output = result;
            exact_rejection(&abi, layouts, &valid, bad, code, message);
        }
        let mut stale = valid.clone();
        stale.events.push(valid.events[14].clone());
        exact_rejection(
            &abi,
            layouts,
            &valid,
            stale,
            "ZRYNA-R3006",
            "handle owner is moved or already released",
        );
        let mut finish = valid.clone();
        finish.events.push(valid.events[12].clone());
        exact_rejection(
            &abi,
            layouts,
            &valid,
            finish,
            "ZRYNA-R3006",
            "last-release receipt is foreign or replayed",
        );
    }
}
