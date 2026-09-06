use super::*;

mod fixtures;
mod joins;
mod payload_construction;
mod upgrade_shape;
mod upgrade_temporary;
use fixtures::{Fixture, Payload};

fn sealed_trace(fixture: &Fixture, raw: raw::Program) -> String {
    let verified = fixture.verify(raw).expect("fully verified handle program");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert!(function.public_export().is_none());
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 3);
    let operations = blocks[0].instructions().collect::<Vec<_>>();
    assert_eq!(
        operations.iter().map(|op| op.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::SharedConstruct,
            VerifiedInstructionKind::SharedClone,
            VerifiedInstructionKind::WeakDowngrade,
            VerifiedInstructionKind::WeakClone,
        ]
    );
    assert_eq!(
        operations.iter().map(|op| op.result().expect("result").index()).collect::<Vec<_>>(),
        [2, 3, 4, 5]
    );
    assert_eq!(
        operations.iter().map(|op| op.result_type().expect("type").index()).collect::<Vec<_>>(),
        [fixture.shared.0, fixture.shared.0, fixture.weak.0, fixture.weak.0]
    );
    assert_eq!(
        operations[0].value_operands().map(super::super::ValueIdentity::index).collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        operations
            .iter()
            .skip(1)
            .map(|op| op.place_operands().next().expect("retained source").index())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let terminator = blocks[0].terminator();
    assert_eq!(terminator.kind(), VerifiedTerminatorKind::WeakUpgradeBranch);
    assert_eq!(terminator.place_operands().next().expect("Weak operand").index(), 4);
    let (success, expired) = terminator.weak_upgrade_edges().expect("two typed outcomes");
    assert_eq!((success.target().index(), expired.target().index()), (1, 2));
    assert_eq!(
        success.arguments().map(super::super::ValueIdentity::index).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        expired.arguments().map(super::super::ValueIdentity::index).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        blocks[1]
            .parameters()
            .map(|value| (value.id().index(), value.ty().index()))
            .collect::<Vec<_>>(),
        [(6, fixture.shared.0), (7, 1)]
    );
    assert_eq!(
        blocks[2]
            .parameters()
            .map(|value| (value.id().index(), value.ty().index()))
            .collect::<Vec<_>>(),
        [(8, 1)]
    );
    let drops = blocks
        .iter()
        .map(|block| {
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(drops, [vec![4, 3, 2, 1], vec![5, 4, 3, 2, 1], vec![4, 3, 2, 1]]);
    let sites = function
        .cleanup_plans()
        .map(|plan| {
            let site = plan.site();
            (site.block().index(), site.instruction_index(), site.role())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sites,
        [
            (0, Some(0), VerifiedCleanupRole::PrepareFailure),
            (0, Some(1), VerifiedCleanupRole::PrepareFailure),
            (0, Some(2), VerifiedCleanupRole::PrepareFailure),
            (0, Some(3), VerifiedCleanupRole::PrepareFailure),
            (0, None, VerifiedCleanupRole::PrepareFailure),
            (1, None, VerifiedCleanupRole::Return),
            (2, None, VerifiedCleanupRole::Return),
        ]
    );
    let prepare = operations
        .iter()
        .map(|op| op.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(&prepare[1..], [vec![1], vec![2, 1], vec![3, 2, 1]]);
    format!("{prepare:?}/{drops:?}/{sites:?}")
}

#[test]
fn shared_weak_operations_and_upgrade_verify_the_complete_payload_type_matrix() {
    for payload in Payload::ALL {
        let fixture = Fixture::new(payload);
        let raw = fixture.program();
        assert_eq!(sealed_trace(&fixture, raw.clone()), sealed_trace(&fixture, raw), "{payload:?}");
    }
}

fn reject_and_replay(fixture: &Fixture, forged: raw::Program, code: &str) {
    let valid = sealed_trace(fixture, fixture.program());
    let first = diagnostic_trace(fixture.verify(forged.clone()).expect_err("forged authority"));
    assert!(first.iter().any(|diagnostic| diagnostic.0 == code), "{first:?}");
    assert_eq!(
        first,
        diagnostic_trace(fixture.verify(forged).expect_err("same ordered rejection"))
    );
    assert_eq!(valid, sealed_trace(fixture, fixture.program()));
}

#[test]
fn shared_weak_instruction_types_reject_wrong_payload_and_handle_categories() {
    for payload in Payload::ALL {
        let fixture = Fixture::new(payload);
        for index in 0..4 {
            let mut forged = fixture.program();
            let f = &mut forged.modules[0].functions[0];
            let wrong = if index < 2 { fixture.weak } else { fixture.shared };
            f.blocks[0].instructions[index].result.as_mut().expect("result").ty = wrong;
            f.places[index + 1].ty = wrong;
            reject_and_replay(&fixture, forged, "ZRYNA-I3005");
        }
        let mut forged = fixture.program();
        forged.modules[0].functions[0].blocks[0].instructions[0].kind =
            raw::InstructionKind::SharedConstruct {
                value: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(0),
            };
        if !matches!(payload, Payload::I32) {
            reject_and_replay(&fixture, forged, "ZRYNA-I3005");
        }
    }
}

#[test]
fn weak_upgrade_rejects_forged_success_expired_and_operand_shapes() {
    let fixture = Fixture::new(Payload::String);
    for mutation in 0..5 {
        let mut forged = fixture.program();
        let f = &mut forged.modules[0].functions[0];
        let raw::Terminator::WeakUpgradeBranch { weak, success, expired, .. } =
            &mut f.blocks[0].terminators[0].kind
        else {
            panic!("upgrade")
        };
        match mutation {
            0 => *weak = raw::PlaceId(1),
            1 => success.arguments.push(raw::ValueId(1)),
            2 => expired.arguments.clear(),
            3 => {
                f.blocks[1].parameters[0].ty = fixture.weak;
                f.places[5].ty = fixture.weak;
            }
            4 => {
                f.blocks[1].parameters[0].ty = raw::TypeId(1);
                f.places[5].ty = raw::TypeId(1);
            }
            _ => unreachable!(),
        }
        reject_and_replay(
            &fixture,
            forged,
            if matches!(mutation, 1 | 2) { "ZRYNA-I3007" } else { "ZRYNA-I3014" },
        );
    }
}

#[test]
fn shared_weak_cleanup_rejects_missing_reordered_returned_and_foreign_owners() {
    let fixture = Fixture::new(Payload::String);
    for plan in 0..7 {
        let mut forged = fixture.program();
        forged.modules[0].functions[0].cleanup_plans[plan].actions.clear();
        reject_and_replay(&fixture, forged, "ZRYNA-I3012");
    }
    for mutation in 0..3 {
        let mut forged = fixture.program();
        let f = &mut forged.modules[0].functions[0];
        match mutation {
            0 => f.cleanup_plans[5].actions.swap(0, 1),
            1 => f.cleanup_plans[6].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(5))),
            2 => {
                f.blocks[0].instructions[1].kind = raw::InstructionKind::SharedClone {
                    place: raw::PlaceId(1),
                    cleanup: raw::CleanupPlanId(0),
                }
            }
            _ => unreachable!(),
        }
        reject_and_replay(&fixture, forged, "ZRYNA-I3012");
    }
}

#[test]
fn shared_weak_owner_reuse_and_exclusive_borrow_conflicts_fail_closed() {
    let fixture = Fixture::new(Payload::String);
    for mutation in 0..3 {
        let mut forged = fixture.program();
        let f = &mut forged.modules[0].functions[0];
        let span = f.span;
        match mutation {
            0 => {
                f.blocks[0].instructions[1].kind = raw::InstructionKind::SharedConstruct {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(1),
                }
            }
            1 => f.blocks[0].instructions.insert(
                1,
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(1) },
                },
            ),
            2 => {
                f.blocks[0]
                    .instructions
                    .insert(1, begin_borrow(0, 1, raw::BorrowAccess::Exclusive, span));
                f.blocks[0].instructions.insert(3, end_borrow(0, span));
            }
            _ => unreachable!(),
        }
        reject_and_replay(&fixture, forged, "ZRYNA-I3010");
    }
}

#[test]
fn weak_clone_and_upgrade_remain_typed_after_both_created_shared_owners_are_dropped() {
    let fixture = Fixture::new(Payload::String);
    let mut raw = fixture.program();
    let f = &mut raw.modules[0].functions[0];
    for (offset, id) in [2, 1].into_iter().enumerate() {
        f.blocks[0].instructions.insert(
            3 + offset,
            raw::Instruction {
                result: None,
                span: f.span,
                kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(id) },
            },
        );
    }
    for (id, owners) in [(3, vec![3]), (4, vec![4, 3]), (5, vec![5, 4, 3]), (6, vec![4, 3])] {
        f.cleanup_plans[id] = fixtures::cleanup(u32::try_from(id).expect("plan"), owners, f.span);
    }
    for _ in 0..2 {
        let verified = fixture.verify(raw.clone()).expect("Weak survives Shared owner releases");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("entry");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[5].kind(), VerifiedInstructionKind::WeakClone);
        assert_eq!(
            instructions[3]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [2]
        );
        assert_eq!(
            instructions[4]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [1]
        );
        assert_eq!(
            instructions[5]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [3]
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [4, 3]
        );
    }
    // IR verifies both CFG successors; this is not runtime proof of expiration.
}

#[test]
fn shared_weak_cleanup_diagnostics_have_exact_order_span_and_valid_replay() {
    let fixture = Fixture::new(Payload::String);
    let mut forged = fixture.program();
    let f = &mut forged.modules[0].functions[0];
    f.cleanup_plans[1].actions.clear();
    f.cleanup_plans[5].actions.clear();
    let span = f.span;
    let expected = (
        "ZRYNA-I3012".to_owned(),
        "cleanup plan is incomplete, duplicated, or out of reverse-completion order".to_owned(),
        Some((span.start(), span.end())),
    );
    assert_eq!(
        diagnostic_trace(fixture.verify(forged.clone()).expect_err("two invalid sites")),
        vec![expected.clone(), expected]
    );
    reject_and_replay(&fixture, forged, "ZRYNA-I3012");
}

#[test]
fn weak_upgrade_ordinary_edge_types_and_success_value_dominance_are_independent() {
    let fixture = Fixture::new(Payload::String);
    let mut forged = fixture.program();
    let raw::Terminator::WeakUpgradeBranch { success, .. } =
        &mut forged.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        panic!("upgrade")
    };
    success.arguments[0] = raw::ValueId(2);
    reject_and_replay(&fixture, forged, "ZRYNA-I3007");
    let mut forged = fixture.program();
    let f = &mut forged.modules[0].functions[0];
    // Success's ordinary i32 parameter has the correct type but cannot be used
    // by the expired successor, which it does not dominate.
    f.blocks[2].terminators[0] = fixtures::returning(7, 6, f.span);
    reject_and_replay(&fixture, forged, "ZRYNA-I3008");
}

#[test]
fn weak_upgrade_return_transfers_synthetic_owner_before_cleanup() {
    let fixture = Fixture::new(Payload::String);
    let mut raw = fixture.program();
    let f = &mut raw.modules[0].functions[0];
    f.result = fixture.shared;
    f.blocks[1].terminators[0] = fixtures::returning(6, 5, f.span);
    f.blocks[2].terminators[0] = fixtures::returning(3, 6, f.span);
    f.cleanup_plans[5] = fixtures::cleanup(5, vec![4, 3, 2, 1], f.span);
    f.cleanup_plans[6] = fixtures::cleanup(6, vec![4, 3, 1], f.span);
    for _ in 0..2 {
        let verified =
            fixture.verify(raw.clone()).expect("each branch transfers one distinct Shared owner");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let drops = function
            .blocks()
            .skip(1)
            .map(|block| {
                block
                    .terminator()
                    .derived_drop_actions()
                    .map(|action| action.root().index())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(drops, [vec![4, 3, 2, 1], vec![4, 3, 1]]);
    }
    let mut forged = raw.clone();
    forged.modules[0].functions[0].cleanup_plans[5]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(5)));
    let first = diagnostic_trace(
        fixture.verify(forged.clone()).expect_err("returned owner cannot be dropped"),
    );
    assert_eq!(first[0].0, "ZRYNA-I3012");
    assert_eq!(first, diagnostic_trace(fixture.verify(forged).expect_err("repeat rejection")));
    fixture.verify(raw).expect("valid return replay after rejection");
}
