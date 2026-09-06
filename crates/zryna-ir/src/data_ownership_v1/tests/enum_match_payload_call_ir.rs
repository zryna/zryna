use super::active_enum_payload_borrow::fixture::{Mode, Seed};
use super::nonindexed_projection_call_scope::enum_call_program;
use super::*;
use crate::data_ownership_v1::{BorrowIdentity, VerifiedCallArgument};

type Trace = Vec<(String, String, Option<(u32, u32)>)>;

fn call_seed(mode: Mode) -> (Seed, raw::Program) {
    let seed = Seed::new(mode);
    let mut raw = enum_call_program(&seed, mode);
    let caller = &mut raw.modules[0].functions[0];
    let span = caller.span;
    let arm = caller.blocks[0].instructions.split_off(1);
    let mut payload_cleanup = caller.cleanup_plans[0].clone();
    payload_cleanup.id = raw::CleanupPlanId(2);
    caller.cleanup_plans.push(payload_cleanup);
    caller.blocks[0].terminators[0].kind = raw::Terminator::EnumMatch {
        place: raw::PlaceId(3),
        arms: (0..2)
            .map(|variant| raw::EnumArm {
                variant,
                edge: raw::Edge { target: raw::BlockId(variant + 1), arguments: vec![] },
            })
            .collect(),
    };
    for (id, instructions, cleanup) in [(1, vec![], 0), (2, arm, 2)] {
        caller.blocks.push(raw::Block {
            id: raw::BlockId(id),
            parameters: vec![],
            instructions,
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(cleanup),
                },
            }],
        });
    }
    (seed, raw)
}

fn trace(seed: &Seed, raw: raw::Program) -> Result<(), Trace> {
    seed.check(raw).map(|_| ()).map_err(diagnostic_trace)
}

fn reject(seed: &Seed, valid: &raw::Program, hostile: raw::Program, code: &str) {
    let first = trace(seed, hostile.clone()).expect_err("hostile match payload call");
    assert!(first.iter().any(|(actual, _, _)| actual == code), "expected {code}: {first:?}");
    assert_eq!(first, trace(seed, hostile).expect_err("deterministic replay"));
    seed.check(valid.clone()).expect("same-candidate recovery");
}

#[test]
fn exhaustive_enum_match_arm_calls_preserve_exact_borrow_region_and_cleanup() {
    for mode in [Mode::Shared, Mode::Exclusive] {
        let (seed, raw) = call_seed(mode);
        let verified = seed.check(raw.clone()).expect("match payload call");
        let caller = verified.modules().next().expect("module").functions().next().expect("caller");
        let blocks = caller.blocks().collect::<Vec<_>>();
        assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
        let instructions = blocks[2].instructions().collect::<Vec<_>>();
        let begin = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::BeginBorrow)
            .expect("begin");
        let call = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::DirectCall)
            .expect("call");
        let end = instructions
            .iter()
            .position(|item| item.kind() == VerifiedInstructionKind::EndBorrow)
            .expect("end");
        assert!(begin < call && call < end);
        assert_eq!(instructions[begin].place_operands().next().expect("payload").index(), 4);
        assert_eq!(
            instructions[call].call_arguments().collect::<Vec<_>>(),
            [VerifiedCallArgument::Borrow(instructions[begin].borrow().expect("borrow identity"))]
        );
        assert_eq!(
            instructions[call]
                .failure_ended_borrows()
                .map(BorrowIdentity::index)
                .collect::<Vec<_>>(),
            [0]
        );
        let cleanup = instructions[call].derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(cleanup[0].active_variant(), Some(1));
        assert_eq!(
            instructions[call]
                .cleanup()
                .and_then(|id| caller.cleanup_plans().find(|plan| plan.id() == id))
                .expect("call cleanup")
                .site()
                .role(),
            VerifiedCleanupRole::CallTrap
        );
        assert_eq!(instructions[end].borrow(), instructions[begin].borrow());
        seed.check(raw).expect("valid replay");
    }
}

#[test]
fn exhaustive_enum_match_calls_reject_arm_access_region_and_cleanup_forgeries() {
    let (seed, valid) = call_seed(Mode::Exclusive);

    let mut inactive = valid.clone();
    let forged = inactive.modules[0].functions[0].blocks[2].instructions.clone();
    inactive.modules[0].functions[0].blocks[1].instructions = forged;
    inactive.modules[0].functions[0].blocks[2].instructions.clear();
    reject(&seed, &valid, inactive, "ZRYNA-I3013");

    let mut wrong_arm = valid.clone();
    let raw::PlaceKind::EnumPayload { variant, .. } =
        &mut wrong_arm.modules[0].functions[0].places[4].kind
    else {
        unreachable!("enum payload place")
    };
    *variant = 0;
    reject(&seed, &valid, wrong_arm, "ZRYNA-I3013");

    let mut access = valid.clone();
    access.modules[0].functions[1].borrow_parameters[0].access = raw::BorrowAccess::Shared;
    reject(&seed, &valid, access, "ZRYNA-I3009");

    let mut region = valid.clone();
    let arm = &mut region.modules[0].functions[0].blocks[2].instructions;
    let end_at = arm
        .iter()
        .position(|instruction| matches!(instruction.kind, raw::InstructionKind::EndBorrow { .. }))
        .expect("end");
    let end = arm.remove(end_at);
    arm.insert(1, end);
    reject(&seed, &valid, region, "ZRYNA-I3011");

    let mut cleanup = valid.clone();
    cleanup.modules[0].functions[0].cleanup_plans[1].actions.remove(0);
    reject(&seed, &valid, cleanup, "ZRYNA-I3012");

    let mut repeated = valid.clone();
    let span = repeated.modules[0].functions[1].span;
    repeated.modules[0].functions[1].borrow_parameters.push(raw::BorrowParameter {
        id: raw::BorrowId(1),
        referent: seed.root,
        access: raw::BorrowAccess::Exclusive,
        span,
    });
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut repeated.modules[0].functions[0].blocks[2].instructions[1].kind
    else {
        unreachable!("direct call")
    };
    arguments.push(raw::CallArgument::Borrow(raw::BorrowId(0)));
    reject(&seed, &valid, repeated, "ZRYNA-I3011");
}
