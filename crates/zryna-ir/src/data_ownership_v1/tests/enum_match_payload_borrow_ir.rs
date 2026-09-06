use super::active_enum_payload_borrow::fixture::{Mode, Seed};
use super::*;

fn match_seed(seed: &Seed) -> raw::Program {
    let mut raw = seed.program();
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let arm = function.blocks[0].instructions.split_off(1);
    let mut payload_cleanup = function.cleanup_plans[0].clone();
    payload_cleanup.id = raw::CleanupPlanId(1);
    function.cleanup_plans.push(payload_cleanup);
    function.blocks[0].terminators[0].kind = raw::Terminator::EnumMatch {
        place: raw::PlaceId(3),
        arms: (0..2)
            .map(|variant| raw::EnumArm {
                variant,
                edge: raw::Edge { target: raw::BlockId(variant + 1), arguments: vec![] },
            })
            .collect(),
    };
    for (id, instructions) in [(1, vec![]), (2, arm)] {
        function.blocks.push(raw::Block {
            id: raw::BlockId(id),
            parameters: vec![],
            instructions,
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(id - 1),
                },
            }],
        });
    }
    raw
}

fn reject(seed: &Seed, valid: &raw::Program, hostile: raw::Program, code: &str) {
    let first = seed.check(hostile.clone()).expect_err("forged refined match borrow");
    assert!(first.iter().any(|error| error.code() == code), "expected {code}: {first:?}");
    assert_eq!(first, seed.check(hostile).expect_err("deterministic replay"));
    seed.check(valid.clone()).expect("same-candidate recovery");
}

#[test]
fn enum_match_arm_borrow_is_bound_to_exact_variant_region_and_cleanup() {
    let seed = Seed::new(Mode::Shared);
    let raw = match_seed(&seed);
    let verified = seed.check(raw.clone()).expect("refined match-arm borrow");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks[0].terminator().kind(), VerifiedTerminatorKind::EnumMatch);
    assert_eq!(
        blocks[0]
            .terminator()
            .enum_arms()
            .map(|arm| (arm.variant(), arm.edge().target().index()))
            .collect::<Vec<_>>(),
        [(0, 1), (1, 2)]
    );
    let instructions = blocks[2].instructions().collect::<Vec<_>>();
    assert_eq!(instructions[0].kind(), VerifiedInstructionKind::BeginBorrow);
    assert_eq!(instructions[1].kind(), VerifiedInstructionKind::EndBorrow);
    assert_eq!(instructions[0].place_operands().next().expect("payload").index(), 4);
    let cleanup = blocks[2].terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup[0].root().index(), 3);
    assert_eq!(cleanup[0].active_variant(), Some(1));
    seed.check(raw).expect("valid replay");
}

#[test]
fn enum_match_payload_borrow_rejects_variant_nominal_region_and_cleanup_forgeries() {
    let seed = Seed::new(Mode::Shared);
    let valid = match_seed(&seed);

    let mut variant = valid.clone();
    let raw::Terminator::EnumMatch { arms, .. } =
        &mut variant.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        unreachable!()
    };
    arms[1].variant = 0;
    reject(&seed, &valid, variant, "ZRYNA-I3014");

    let mut nominal = valid.clone();
    let function = &mut nominal.modules[0].functions[0];
    function.parameters[1].ty = seed.string;
    function.places[1].ty = seed.string;
    function.places[3].ty = raw::TypeId(4);
    function.blocks[0].instructions[0].result.as_mut().expect("enum result").ty = raw::TypeId(4);
    let raw::InstructionKind::EnumConstruct { variant, payload, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        unreachable!()
    };
    *variant = 0;
    *payload = Some(raw::ValueId(1));
    for plan in &mut function.cleanup_plans {
        plan.actions.retain(|action| *action != raw::DropAction::DropPlace(raw::PlaceId(1)));
    }
    reject(&seed, &valid, nominal, "ZRYNA-I3006");

    let mut region = valid.clone();
    region.modules[0].functions[0].blocks[2].instructions.pop();
    reject(&seed, &valid, region, "ZRYNA-I3011");

    let mut cleanup = valid.clone();
    cleanup.modules[0].functions[0].cleanup_plans[1].actions.remove(0);
    reject(&seed, &valid, cleanup, "ZRYNA-I3012");
}
