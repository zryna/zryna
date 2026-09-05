use super::mixed_replacement_authority_fixture::{authorities, seed};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn mixed_replacement_seals_old_payload_retention_commit_and_new_pending_order() {
    let (sources, linear, linux) = authorities();
    let raw = seed(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let mut previous = None;
    for _ in 0..2 {
        let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect("complete independent mixed replacement control");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 7);
        let prepared = instructions[4].derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            prepared.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [5, 4, 1]
        );
        assert_eq!(prepared[1].active_variant(), Some(1));
        assert_eq!(prepared[1].moved_projections().count(), 0);

        let replace = instructions[6];
        assert_eq!(replace.kind(), VerifiedInstructionKind::ReplacePlace);
        assert_eq!(replace.result(), None);
        let old = replace.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].root().index(), 4);
        assert_eq!(old[0].kind(), VerifiedDropActionKind::Place);
        assert_eq!(old[0].active_variant(), Some(1));
        assert_eq!(old[0].moved_projections().count(), 0);

        // The sealed active payload is a complete Vec<String> container, not a prefix or handle.
        let choice = linear.types().find(|ty| ty.category() == TypeCategory::Enum).expect("Enum");
        let payload = choice.variants()[1].payload().expect("old active payload");
        let vector = linear.type_by_id(payload).expect("payload layout");
        assert_eq!(vector.category(), TypeCategory::Vec);
        assert_eq!(
            linear
                .type_by_id(vector.referenced_type().expect("element"))
                .expect("String")
                .category(),
            TypeCategory::String
        );
        let remaining = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            remaining.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [4, 5]
        );
        assert_eq!(remaining[0].active_variant(), Some(0));
        assert_eq!(remaining[0].moved_projections().count(), 0);
        assert_eq!(remaining[1].active_variant(), None);
        assert_eq!(
            function.cleanup_plans().map(|plan| plan.site().role()).collect::<Vec<_>>(),
            [
                VerifiedCleanupRole::PrepareFailure,
                VerifiedCleanupRole::PrepareFailure,
                VerifiedCleanupRole::PrepareFailure,
                VerifiedCleanupRole::Return,
            ]
        );
        let observed = (prepared, old, remaining);
        if let Some(prior) = previous.replace(observed.clone()) {
            assert_eq!(observed, prior, "complete sealed cleanup replay");
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Damage {
    WrongType,
    ConsumedOldOwner,
    MovedDestination,
    ReusedPreparedOwner,
    StaleCompletionOrder,
    MissingRetainedDestination,
}

fn mutate(program: &mut raw::Program, damage: Damage) -> &'static str {
    let function = &mut program.modules[0].functions[0];
    match damage {
        Damage::WrongType | Damage::ConsumedOldOwner => {
            let raw::InstructionKind::ReplacePlace { value, .. } =
                &mut function.blocks[0].instructions[6].kind
            else {
                panic!("replacement")
            };
            *value = raw::ValueId(if matches!(damage, Damage::WrongType) { 2 } else { 4 });
            if matches!(damage, Damage::WrongType) { "ZRYNA-I3005" } else { "ZRYNA-I3010" }
        }
        Damage::MovedDestination => {
            let span = function.span;
            let ty = function.places[4].ty;
            function.places.push(raw::Place {
                id: raw::PlaceId(8),
                ty,
                span,
                kind: raw::PlaceKind::Temporary(raw::ValueId(8)),
            });
            function.blocks[0].instructions.insert(
                6,
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(8), ty, span }),
                    span,
                    kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(4) },
                },
            );
            "ZRYNA-I3010"
        }
        Damage::ReusedPreparedOwner => {
            let second = function.blocks[0].instructions[6].clone();
            function.blocks[0].instructions.push(second);
            "ZRYNA-I3010"
        }
        Damage::StaleCompletionOrder => {
            function.cleanup_plans[3].actions.swap(0, 1);
            "ZRYNA-I3012"
        }
        Damage::MissingRetainedDestination => {
            function.cleanup_plans[2].actions.remove(1);
            "ZRYNA-I3012"
        }
    }
}

#[test]
fn mixed_replacement_hostile_mutations_reject_deterministically_after_valid_control() {
    let (sources, linear, linux) = authorities();
    let seed = seed(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(seed.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("independently valid baseline before every isolated mutation");
    for damage in [
        Damage::WrongType,
        Damage::ConsumedOldOwner,
        Damage::MovedDestination,
        Damage::ReusedPreparedOwner,
        Damage::StaleCompletionOrder,
        Damage::MissingRetainedDestination,
    ] {
        let mut raw = seed.clone();
        let expected_code = mutate(&mut raw, damage);
        let first = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect_err("hostile replacement");
        let second = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect_err("repeat hostile replacement");
        assert_eq!(first, second, "complete ordered diagnostic replay: {damage:?}");
        assert_eq!(first[0].code(), expected_code, "first authority rejection: {damage:?}");
    }
}
