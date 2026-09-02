use super::*;

#[test]
fn partial_struct_assignment_prepares_then_replaces_and_returns_the_exact_mask() {
    let (source, raw) = owned_pair_partial_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf = roots[&1];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let assignment_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial assignment preparation");
    let replace = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::ReplacePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assignment commit");
    let return_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial assigned-root return");
    assert!(assignment_move < replace && replace < return_move);
    assert_eq!(
        instructions[replace]
            .derived_drop_actions()
            .map(|action| action.root())
            .collect::<Vec<_>>(),
        [target_root],
        "the fully initialized old destination is dropped exactly at commit",
    );
    let assignment_value = instructions[assignment_move].result().expect("assignment value");
    let assignment_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == assignment_value)
        })
        .expect("partial assignment temporary")
        .id();
    let returned_value = instructions[return_move].result().expect("return value");
    let return_temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned_value)
        })
        .expect("partial return temporary")
        .id();
    let fields = |root| {
        function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    for root in [source_root, assignment_temporary, target_root, return_temporary] {
        assert_eq!(fields(root).keys().copied().collect::<Vec<_>>(), [0, 1]);
    }
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [moved_leaf],
        "only the moved String leaf survives reverse return cleanup",
    );
}

#[test]
fn partial_struct_assignment_invalidates_the_source_owner() {
    let (source, raw) = owned_pair_partial_assignment_old_source_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful old assignment source");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("assigned source must move");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("assigned source replay");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "aggregate value 'p' is moved or only partially available",
    );
}

#[test]
fn partial_struct_assignment_rejects_a_partial_destination_before_rhs_mutation() {
    let (source, raw) = owned_pair_partial_self_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial destination");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("partial destination");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("partial destination replay");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "owned aggregate assignment target is immutable, moved, or only partially available",
    );
}
