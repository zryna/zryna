use super::finite_recursive_fixture::{PROJECTION_SOURCE, projection_snapshot};
use super::*;
use crate::data_ownership_v1::VerifiedProgram;
use zryna_ir::data_ownership_v1::{
    PlaceIdentity, VerifiedDropAction, VerifiedGenericCloneSource, VerifiedInstructionKind,
    VerifiedPlaceKind,
};

fn verified_pair() -> (VerifiedProgram, VerifiedProgram) {
    let sources = sources_for(PROJECTION_SOURCE);
    let syntax = verify_snapshot(projection_snapshot(), &sources)
        .expect("authenticated finite recursive projection source");
    let lower = || {
        lower(pair_input(&syntax, &sources))
            .unwrap_or_else(|errors| panic!("finite recursive projections: {errors:#?}"))
    };
    (lower(), lower())
}

fn roots(actions: impl Iterator<Item = VerifiedDropAction>) -> Vec<u32> {
    actions.map(|drop| drop.root().index()).collect()
}

fn exact_recursive_clone(
    instruction: FaultVerifiedInstruction<'_>,
    source: u32,
    result: u32,
    destination: u32,
) {
    let clone = instruction.generic_clone().expect("recursive clone");
    assert!(
        matches!(clone.source(), VerifiedGenericCloneSource::Place(place) if place.index() == source)
    );
    assert_eq!(clone.result().index(), result);
    assert_eq!(clone.destination().index(), destination);
    assert_eq!(clone.frontier().types().count(), 3, "String/Node/Vec visited once");
}

#[test]
fn finite_recursive_static_move_and_replacement_bind_exact_owners_masks_and_commit() {
    let program = verified_pair().0;
    let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
    assert_eq!(functions.len(), 6);
    for function_at in [0, 2] {
        let function = functions[function_at];
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[2].kind(), VerifiedInstructionKind::GenericMoveFromPlace);
        assert_eq!(instructions[2].place_operands().next().expect("projection").index(), 3);
        assert_eq!(instructions[2].result().expect("move result").index(), 2);
        assert!(matches!(
            function.places().find(|place| place.id().index() == 4).expect("result owner").kind(),
            VerifiedPlaceKind::Temporary(value) if value.index() == 2
        ));
        assert_eq!(block.terminator().value_operands().next().expect("returned move").index(), 2);
        let retained = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(roots(retained.iter().cloned()), [2]);
        assert_eq!(
            retained[0].moved_projections().map(PlaceIdentity::index).collect::<Vec<_>>(),
            [3]
        );
    }
    for function_at in [1, 3] {
        let function = functions[function_at];
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        exact_recursive_clone(instructions[2], 1, 3, 5);
        assert_eq!(roots(instructions[2].derived_drop_actions()), [1, 3]);
        let commit = instructions[3];
        assert_eq!(commit.kind(), VerifiedInstructionKind::GenericReplacePlace);
        assert_eq!(commit.place_operands().next().expect("old target").index(), 4);
        assert_eq!(commit.value_operands().next().expect("prepared owner").index(), 3);
        let old = commit.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(roots(old.iter().cloned()), [4]);
        assert_eq!(old[0].active_variant(), None);
        assert_eq!(old[0].moved_projections().count(), 0);
        assert_eq!(instructions[4].result().expect("return producer").index(), 4);
        assert_eq!(block.terminator().value_operands().next().expect("return").index(), 4);
        assert_eq!(roots(block.terminator().derived_drop_actions()), [1]);
    }
}

#[test]
fn finite_recursive_vec_observation_and_replacement_bind_borrows_and_exact_owners() {
    let (program, replay) = verified_pair();
    let functions = program.modules().next().expect("module").functions().collect::<Vec<_>>();
    let observed = functions[4].blocks().next().expect("observation block");
    let instructions = observed.instructions().collect::<Vec<_>>();
    let begin = instructions[3].indexed_borrow().expect("bounds authority");
    let clone = instructions[4].generic_clone().expect("recursive indexed clone");
    assert!(
        matches!(clone.source(), VerifiedGenericCloneSource::Borrow(id) if id == begin.borrow())
    );
    assert_eq!((clone.destination().index(), clone.result().index()), (4, 4));
    assert_eq!(clone.frontier().types().count(), 3);
    assert_eq!(roots(instructions[3].derived_drop_actions()), [3]);
    assert_eq!(roots(instructions[4].derived_drop_actions()), [3]);
    assert_eq!(observed.terminator().value_operands().next().expect("observed result").index(), 4);
    assert_eq!(roots(observed.terminator().derived_drop_actions()), [3]);

    let replaced = functions[5].blocks().next().expect("replacement block");
    let instructions = replaced.instructions().collect::<Vec<_>>();
    let begin = instructions[3].indexed_borrow().expect("exclusive bounds authority");
    exact_recursive_clone(instructions[4], 2, 5, 5);
    assert_eq!(roots(instructions[4].derived_drop_actions()), [2, 4]);
    let commit = instructions[5].borrow_replacement().expect("exact replacement commit");
    assert_eq!(commit.borrow(), begin.borrow());
    assert_eq!(commit.value().index(), 5);
    assert_eq!(commit.old_value_drop().referent(), commit.referent());
    assert_eq!(instructions[7].result().expect("return producer").index(), 6);
    assert_eq!(replaced.terminator().value_operands().next().expect("return").index(), 6);
    assert_eq!(roots(replaced.terminator().derived_drop_actions()), [2]);

    assert_eq!(format!("{program:#?}"), format!("{replay:#?}"), "stable replay");
}
