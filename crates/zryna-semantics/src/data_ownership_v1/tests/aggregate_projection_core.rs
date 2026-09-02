use super::*;

#[test]
fn owned_struct_projections_copy_and_move_with_a_root_relative_cleanup_mask() {
    let (source, raw) = owned_pair_projected_return_snapshot("first");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful owned Struct projections");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Struct projections");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let root = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Local(0)))
        .expect("owned Pair root")
        .id();
    let first = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == root
            )
        })
        .expect("String field projection")
        .id();
    let flag = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 1 } if base == root
            )
        })
        .expect("Copy field projection")
        .id();
    let block = function.blocks().next().expect("block");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::CopyFromPlace
            && instruction.place_operands().next() == Some(flag)
    }));
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next() == Some(first)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == root)
        .expect("partially moved root cleanup");
    assert_eq!(
        cleanup.moved_projections().map(FaultPlaceIdentity::index).collect::<Vec<_>>(),
        vec![first.index()],
    );
    assert_eq!(
        cleanup.initialized_projections().map(FaultPlaceIdentity::index).collect::<Vec<_>>(),
        vec![flag.index()],
    );
}
