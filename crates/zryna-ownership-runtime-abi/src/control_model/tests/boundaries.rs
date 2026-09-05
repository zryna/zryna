use super::*;

#[test]
fn control_model_requires_exact_independent_surviving_owner_contract() {
    let (abi, layouts, _) = authorities();
    let input = trace(&abi, &layouts, vec![seed(&abi, &layouts)]);
    let error = verify_control_trace(&abi, &layouts, 17, &[], &input).expect_err("omitted release");
    assert_eq!(error.code(), "ZRYNA-R3006");
    assert_eq!(
        error.message(),
        "surviving external owners differ from the independent exit contract"
    );
    for expected in [
        ExpectedOwner { owner: 0, control: 0, weak: true },
        ExpectedOwner { owner: 0, control: 1, weak: false },
        ExpectedOwner { owner: 1, control: 0, weak: false },
    ] {
        assert_eq!(
            verify_control_trace(&abi, &layouts, 17, &[expected], &input)
                .expect_err("wrong survivor"),
            error
        );
    }
    verify_control_trace(
        &abi,
        &layouts,
        17,
        &[ExpectedOwner { owner: 0, control: 0, weak: false }],
        &input,
    )
    .expect("explicit returned owner");
}

#[test]
fn control_model_payload_shapes_derive_reverse_active_cleanup_and_vec_storage_last() {
    let (abi, linear, linux) = authorities();
    for layouts in [&linear, &linux] {
        let string = ty(layouts, TypeCategory::String);
        for record in layouts.types().filter(|record| {
            matches!(
                record.category(),
                TypeCategory::Bool
                    | TypeCategory::I32
                    | TypeCategory::String
                    | TypeCategory::Enum
                    | TypeCategory::FixedArray
                    | TypeCategory::Vec
            )
        }) {
            let cases = if record.category() == TypeCategory::Enum {
                vec![0, 1]
            } else if record.category() == TypeCategory::Vec {
                vec![0, 2]
            } else {
                vec![0]
            };
            for count in cases {
                let mut nodes = vec![];
                let mut drops = vec![];
                let kind = match record.category() {
                    TypeCategory::Bool | TypeCategory::I32 => PayloadKind::Copy,
                    TypeCategory::String => {
                        drops.push(0);
                        PayloadKind::String
                    }
                    TypeCategory::Enum => {
                        if count == 1 {
                            nodes.push(PayloadNode { ty: string, kind: PayloadKind::String });
                            drops.push(0);
                        }
                        PayloadKind::Variant(count, (count == 1).then_some(0))
                    }
                    TypeCategory::FixedArray | TypeCategory::Vec => {
                        let length =
                            u32::try_from(record.array_length().unwrap_or(u64::from(count)))
                                .expect("small array");
                        for _ in 0..length {
                            nodes.push(PayloadNode { ty: string, kind: PayloadKind::String });
                        }
                        drops.extend((0..length).rev());
                        if record.category() == TypeCategory::Vec {
                            drops.push(length);
                        }
                        PayloadKind::Children((0..length).collect())
                    }
                    _ => unreachable!(),
                };
                nodes.push(PayloadNode { ty: record.id(), kind });
                let mut events = vec![
                    construct(&abi, layouts, nodes, 0, 0, 1024),
                    handle(
                        LogicalOperation::StrongReleaseBegin,
                        0,
                        None,
                        Some(true),
                        state(0, 1, true, true),
                    ),
                ];
                events.extend(
                    drops
                        .iter()
                        .map(|&node| ControlEventKind::DropPayloadNode { control: 0, node }),
                );
                events.push(ControlEventKind::Finish {
                    control: 0,
                    after: state(0, 0, false, false),
                });
                let input = trace(&abi, layouts, events);
                assert_eq!(
                    verify_control_trace(&abi, layouts, 17, &[], &input)
                        .expect("typed payload cleanup")
                        .live_owners(),
                    0
                );
                if !drops.is_empty() {
                    let mut missing = input.clone();
                    missing.events.remove(2);
                    for (site, event) in missing.events.iter_mut().enumerate() {
                        event.site = site as u64;
                    }
                    reject(&abi, layouts, &missing);
                }
            }
        }
    }
}

#[test]
fn control_model_allocation_failure_retains_embedded_handle_until_successful_retry() {
    let (abi, layouts, _) = authorities();
    let shared = layouts
        .types()
        .find(|record| {
            record.category() == TypeCategory::Shared
                && record.referenced_type() == Some(ty(&layouts, TypeCategory::String))
        })
        .expect("Shared String")
        .id();
    let nested = construct(
        &abi,
        &layouts,
        vec![PayloadNode { ty: shared, kind: PayloadKind::Handle(0) }],
        1,
        1,
        2048,
    );
    let mut failed = nested.clone();
    if let ControlEventKind::Construct { status, base, control, owner, .. } = &mut failed {
        *status = RuntimeStatus::Allocation;
        *base = 0;
        *control = None;
        *owner = None;
    }
    let input = trace(&abi, &layouts, vec![seed(&abi, &layouts), failed, nested]);
    let expected = [ExpectedOwner { owner: 1, control: 1, weak: false }];
    assert_eq!(
        verify_control_trace(&abi, &layouts, 17, &expected, &input)
            .expect("payload transferred only on retry success")
            .live_owners(),
        2
    );
    let mut stale = input.clone();
    stale.events.push(ControlEvent {
        site: 3,
        kind: handle(LogicalOperation::StrongClone, 0, Some(2), None, state(2, 1, false, true)),
    });
    reject(&abi, &layouts, &stale);
}

#[test]
fn control_model_count_boundaries_remain_indivisible_existing_abi_claims() {
    // Synthetic count proofs, not billions of issued owners or executed runtime outcomes.
    for operation in [LogicalOperation::StrongClone, LogicalOperation::WeakUpgrade] {
        for (before_count, status, after_count) in [
            (u32::MAX - 1, RuntimeStatus::Ok, u32::MAX),
            (u32::MAX, RuntimeStatus::Refcount, u32::MAX),
        ] {
            let before = state(before_count, 2, false, true);
            let after = state(after_count, 2, false, true);
            validate_transition(TransitionClaim::Control {
                operation,
                before,
                status,
                bool_result: None,
                after,
            })
            .expect("exact atomic increment/overflow");
            let forged = ControlState { strong_count: 0, ..after };
            assert!(
                validate_transition(TransitionClaim::Control {
                    operation,
                    before,
                    status,
                    bool_result: None,
                    after: forged
                })
                .is_err()
            );
        }
    }
    let expired = state(0, 1, false, false);
    validate_transition(TransitionClaim::Control {
        operation: LogicalOperation::WeakUpgrade,
        before: expired,
        status: RuntimeStatus::Expired,
        bool_result: None,
        after: expired,
    })
    .expect("expired neither increments nor produces success");
    for maximum in [MAX_STATUS_TRANSITIONS, MAX_ALLOCATION_OPERATIONS, MAX_LIVE_ALLOCATIONS] {
        assert_eq!(
            checked_model_count(maximum - 1, 1, maximum).expect("exact scalar budget"),
            maximum
        );
        assert_eq!(
            checked_model_count(maximum, 1, maximum).expect_err("first extra").code(),
            "ZRYNA-R3201"
        );
        assert_eq!(
            checked_model_count(u64::MAX, 1, maximum).expect_err("checked overflow").code(),
            "ZRYNA-R3201"
        );
        assert_eq!(checked_model_count(0, 1, maximum).expect("recovery"), 1);
    }
}
