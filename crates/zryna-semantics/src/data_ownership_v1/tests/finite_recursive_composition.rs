use super::finite_recursive_fixture::{SOURCE, snapshot};
use super::*;
use zryna_ir::data_ownership_v1::{
    PlaceIdentity, VerifiedDropAction, VerifiedDropActionKind, VerifiedGenericCloneSource,
    VerifiedInstructionKind, VerifiedPlaceKind,
};
use zryna_layout::TypeCategory;

fn masks_are_empty(actions: &[VerifiedDropAction]) -> bool {
    actions.iter().all(|drop| drop.moved_projections().len() == 0)
}

const EXPECTED_KINDS: [VerifiedInstructionKind; 23] = [
    VerifiedInstructionKind::StringFromUtf8,
    VerifiedInstructionKind::EnumConstruct,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::MoveFromPlace,
    VerifiedInstructionKind::VecConstruct,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::MoveFromPlace,
    VerifiedInstructionKind::EnumConstruct,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::MoveFromPlace,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::StringFromUtf8,
    VerifiedInstructionKind::EnumConstruct,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::MoveFromPlace,
    VerifiedInstructionKind::ReplacePlace,
    VerifiedInstructionKind::GenericClonePlace,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::VecConstruct,
    VerifiedInstructionKind::InitializePlace,
    VerifiedInstructionKind::GenericClonePlace,
    VerifiedInstructionKind::VecPush,
    VerifiedInstructionKind::MoveFromPlace,
];

#[test]
#[allow(clippy::too_many_lines)]
fn finite_recursive_composition_reaches_verified_ir() {
    let sources = sources_for(SOURCE);
    let syntax = verify_snapshot(snapshot(), &sources).expect("authenticated recursive source");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("finite recursive composition");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
            EXPECTED_KINDS
        );

        for (instruction_at, source, result, destination) in
            [(9, 8, 6, 9), (14, 10, 9, 14), (22, 16, 13, 20)]
        {
            let moved = instructions[instruction_at];
            assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
            assert_eq!(moved.place_operands().next().expect("exact move source").index(), source);
            assert_eq!(moved.result().expect("exact move result").index(), result);
            assert!(matches!(
                function
                    .places()
                    .find(|place| place.id().index() == destination)
                    .expect("move destination")
                    .kind(),
                VerifiedPlaceKind::Temporary(value) if value.index() == result
            ));
        }
        let replacement = instructions[15];
        assert_eq!(replacement.place_operands().next().expect("old target").index(), 13);
        assert_eq!(replacement.value_operands().next().expect("prepared move").index(), 9);
        let old = replacement.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].active_variant(), Some(0), "old leaf variant drops at commit");
        assert!(masks_are_empty(&old));

        let clone_instructions = [instructions[16], instructions[20]];
        let mut clones = Vec::new();
        for clone_instruction in clone_instructions {
            let clone = clone_instruction.generic_clone().expect("recursive structural clone");
            let VerifiedGenericCloneSource::Place(source) = clone.source() else {
                panic!("source clone must retain one exact place");
            };
            assert_ne!(source, clone.destination());
            let types = clone.frontier().types().collect::<Vec<_>>();
            assert_eq!(types.len(), 3, "the recursive type cycle is visited once");
            assert!(types.windows(2).all(|pair| pair[0].id().index() < pair[1].id().index()));
            let node = types
                .iter()
                .find(|ty| ty.category() == TypeCategory::Enum)
                .expect("recursive Node enum");
            let children = node.variants()[1].payload().expect("branch payload");
            let vector = types.iter().find(|ty| ty.id() == children).expect("children Vec");
            assert_eq!(vector.referenced_type(), Some(node.id()), "Vec closes the type cycle");

            let prepare = clone_instruction.derived_drop_actions().collect::<Vec<_>>();
            let prefix =
                clone_instruction.generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
            assert_eq!(prefix.len(), prepare.len() + 1);
            assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
            assert_eq!(prefix[0].root(), clone.destination());
            assert_eq!(&prefix[1..], prepare.as_slice());
            assert!(prepare.iter().any(|drop| drop.root() == source));
            assert!(masks_are_empty(&prepare));
            assert_eq!(
                prepare
                    .iter()
                    .find(|drop| drop.root() == source)
                    .and_then(VerifiedDropAction::active_variant),
                Some(1),
                "the nonempty recursive branch remains active while its clone is prepared"
            );
            clones.push((clone, source, prepare));
        }

        let push = instructions[21];
        let pushed = clones[1].0;
        assert_eq!(push.value_operands().collect::<Vec<_>>(), [pushed.result()]);
        let vector = push.place_operands().next().expect("forest owner");
        let push_cleanup = push.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            push_cleanup.iter().map(VerifiedDropAction::root).collect::<Vec<_>>(),
            [pushed.destination(), vector, clones[1].1, clones[0].1],
            "push failure first releases its prepared clone, then prior roots in reverse order"
        );
        assert!(masks_are_empty(&push_cleanup));
        assert_eq!(block.terminator().value_operands().next().expect("return").index(), 13);
        let returned_cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            returned_cleanup
                .iter()
                .map(VerifiedDropAction::root)
                .map(PlaceIdentity::index)
                .collect::<Vec<_>>(),
            [18, 13],
            "returned clone is excluded while target and pushed forest remain"
        );
        assert!(masks_are_empty(&returned_cleanup));
        let observation = format!("{program:?}");
        if let Some(previous) = previous.replace(observation.clone()) {
            assert_eq!(previous, observation, "verified recursive lowering replays exactly");
        }
    }
}
