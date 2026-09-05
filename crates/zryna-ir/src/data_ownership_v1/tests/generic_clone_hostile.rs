use super::generic_clone_fixture::{Fixture, GraphKind};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn generic_clone_forbidden_descendants_propagate_through_nested_and_recursive_graphs() {
    for mode in [GraphKind::Shared, GraphKind::Weak, GraphKind::RecursiveShared] {
        let fixture = Fixture::with_graph(TypeCategory::Vec, mode);
        let raw = fixture.seed();
        let mut control = raw.clone();
        let function = &mut control.modules[0].functions[0];
        function.blocks[0].instructions[0].kind =
            raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) };
        function.cleanup_plans.truncate(1);
        function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(1))];
        if let raw::Terminator::Return { cleanup, .. } = &mut function.blocks[0].terminators[0].kind
        {
            *cleanup = raw::CleanupPlanId(0);
        }
        fixture.verify(control);
        fixture.rejects(raw, "ZRYNA-I3005");
    }
}

#[test]
fn generic_clone_zero_stride_vec_is_rejected_by_both_layout_authorities_before_ir() {
    let fixture = Fixture::new(TypeCategory::Vec);
    fixture.verify(fixture.seed());
    let graph = generic_clone_fixture::graph(&fixture.sources, GraphKind::ZeroStrideVec);
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let check = || {
            zryna_layout::verify(&graph, &fixture.sources, target)
                .expect_err("zero-stride Vec has no sealed layout authority")
        };
        let first = check();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].code(), "ZRYNA-L3003");
        assert_eq!(first, check());
    }
}

#[test]
fn generic_clone_rejects_forged_recursive_prefix_owner_shape_order_and_site() {
    for category in
        [TypeCategory::Struct, TypeCategory::Enum, TypeCategory::FixedArray, TypeCategory::Vec]
    {
        let fixture = Fixture::new(category);
        let seed = fixture.seed();
        fixture.verify(seed.clone());
        for (mutation, label, code) in [
            (0, "missing destination prefix", "ZRYNA-I3013"),
            (1, "whole drop instead of recursive prefix", "ZRYNA-I3013"),
            (2, "source prefix duplicates pending source", "ZRYNA-I3012"),
            (3, "legacy aggregate prefix instead of generic frontier", "ZRYNA-I3013"),
            (4, "reversed pending survivor order", "ZRYNA-I3013"),
            (5, "duplicate pending source drop", "ZRYNA-I3012"),
            (6, "destination prefix on ordinary prepare role", "ZRYNA-I3012"),
            (7, "prepare and prefix roles reuse one plan", "ZRYNA-I3012"),
        ] {
            let mut raw = seed.clone();
            let function = &mut raw.modules[0].functions[0];
            let prefix = &mut function.cleanup_plans[1].actions;
            match mutation {
                0 => {
                    prefix.remove(0);
                }
                1 => prefix[0] = raw::DropAction::DropPlace(raw::PlaceId(2)),
                2 => {
                    prefix[0] = raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(0));
                }
                3 => prefix[0] = raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(2)),
                4 => prefix.swap(1, 2),
                5 => prefix.push(raw::DropAction::DropPlace(raw::PlaceId(0))),
                6 => function.cleanup_plans[0]
                    .actions
                    .insert(0, raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(2))),
                7 => {
                    let raw::InstructionKind::GenericClonePlace { prefix_cleanup, .. } =
                        &mut function.blocks[0].instructions[0].kind
                    else {
                        panic!("clone");
                    };
                    *prefix_cleanup = raw::CleanupPlanId(0);
                }
                _ => unreachable!("eight cases"),
            }
            fixture.rejects_case(raw, code, &format!("{category:?}: {label}"));
        }
    }
}

#[test]
fn generic_clone_rejects_wrong_result_type_and_unavailable_or_exclusive_source() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let seed = fixture.seed();
    fixture.verify(seed.clone());
    let mut wrong = seed.clone();
    let function = &mut wrong.modules[0].functions[0];
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").ty = fixture.string;
    function.places[2].ty = fixture.string;
    function.result = fixture.string;
    fixture.rejects(wrong, "ZRYNA-I3005");

    let mut exclusive = seed.clone();
    let function = &mut exclusive.modules[0].functions[0];
    let span = function.span;
    function.blocks[0]
        .instructions
        .insert(0, begin_borrow(0, 0, raw::BorrowAccess::Exclusive, span));
    function.blocks[0].instructions.push(end_borrow(0, span));
    fixture.rejects(exclusive, "ZRYNA-I3010");

    let mut moved = seed;
    let function = &mut moved.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.root,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    function.places[2].kind = raw::PlaceKind::Temporary(raw::ValueId(4));
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").id = raw::ValueId(4);
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: fixture.root, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    );
    for plan in &mut function.cleanup_plans {
        for action in &mut plan.actions {
            if *action == raw::DropAction::DropPlace(raw::PlaceId(0)) {
                *action = raw::DropAction::DropPlace(raw::PlaceId(3));
            }
        }
        let end = plan.actions.len();
        plan.actions.swap(end - 1, end - 2);
    }
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(4);
    }
    fixture.rejects(moved, "ZRYNA-I3010");
}

#[test]
fn generic_clone_shared_source_stays_available_and_repeated_clones_get_distinct_owners() {
    let fixture = Fixture::new(TypeCategory::Vec);
    let mut raw = fixture.seed();
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].instructions.insert(0, begin_borrow(0, 0, raw::BorrowAccess::Shared, span));
    function.blocks[0].instructions.push(end_borrow(0, span));
    fixture.verify(raw.clone());
    let function = &mut raw.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.root,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
    });
    function.blocks[0].instructions.insert(
        2,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: fixture.root, span }),
            span,
            kind: raw::InstructionKind::GenericClonePlace {
                place: raw::PlaceId(0),
                cleanup: raw::CleanupPlanId(2),
                prefix_cleanup: raw::CleanupPlanId(3),
            },
        },
    );
    let pending = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    function.cleanup_plans[2].actions = pending.clone();
    let mut prefix = vec![raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(3))];
    prefix.extend(pending.clone());
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(3),
        span,
        actions: prefix,
    });
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(4),
        span,
        actions: pending,
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(4), cleanup: raw::CleanupPlanId(4) };
    let verified = fixture.verify(raw);
    let block = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block");
    let clones = block
        .instructions()
        .filter(|i| i.kind() == VerifiedInstructionKind::GenericClonePlace)
        .collect::<Vec<_>>();
    assert_eq!(clones.len(), 2);
    assert_ne!(clones[0].result(), clones[1].result());
    assert_eq!(
        clones[0].place_operands().collect::<Vec<_>>(),
        clones[1].place_operands().collect::<Vec<_>>()
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|drop| drop.root().index())
            .collect::<Vec<_>>(),
        [2, 1, 0]
    );
}
