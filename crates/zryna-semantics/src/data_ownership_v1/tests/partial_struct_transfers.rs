use super::*;

#[test]
fn partial_struct_owner_transfers_through_temporary_into_one_local() {
    let (source, raw) = owned_pair_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial local transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct local transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let source_first = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal }
                    if base == source_root && ordinal == 0
            )
        })
        .expect("source first field")
        .id();
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let projected_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_first)
        })
        .expect("projected String move");
    let whole_move = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial whole-root move");
    let initialize = instructions
        .iter()
        .position(|instruction| {
            instruction.kind() == VerifiedInstructionKind::InitializePlace
                && instruction.place_operands().next() == Some(target_root)
        })
        .expect("partial target initialization");
    assert!(projected_move < whole_move && whole_move < initialize);
    let transfer_value = instructions[whole_move].result().expect("whole transfer value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == transfer_value)
        })
        .expect("partial transfer temporary")
        .id();
    for root in [source_root, temporary, target_root] {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => Some(ordinal),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(fields, [0, 1], "complete exact topology for root {root:?}");
    }
    let target_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == target_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == target_root)
        .expect("transferred partial target cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_fields[&0]]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_fields[&1]]);
    assert!(
        block
            .terminator()
            .derived_drop_actions()
            .all(|action| { action.root() != source_root && action.root() != temporary })
    );
}

#[test]
fn partial_struct_transfer_invalidates_the_old_source_owner() {
    let (source, raw) = owned_pair_partial_transfer_then_use_source_snapshot();
    let use_start = source.find("flag: p.flag").expect("old source use") + "flag: ".len();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful old source use");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("old source must be moved");
    let replay = lower(pair_input(&syntax, &sources)).expect_err("old source replay must be moved");
    assert_eq!(diagnostics, replay);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3014");
    assert_eq!(
        diagnostics[0].message(),
        "owned projection is unavailable or overlaps an already moved subobject"
    );
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(
            &sources,
            zryna_source::UntrustedSpan {
                file: 0,
                start: u32::try_from(use_start).expect("old source offset"),
                end: u32::try_from(use_start + 6).expect("old source end"),
            },
        ))
    );
}

#[test]
fn partial_struct_owner_can_transfer_repeatedly_without_mask_drift() {
    let (source, raw) = owned_pair_repeated_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful repeated partial transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("repeated partial transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let final_root = roots[&3];
    let final_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == final_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let block = function.blocks().next().expect("block");
    assert_eq!(
        block
            .instructions()
            .filter(|instruction| {
                instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                    && instruction
                        .place_operands()
                        .next()
                        .is_some_and(|place| place == roots[&0] || place == roots[&2])
            })
            .count(),
        2
    );
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == final_root)
        .expect("final repeated-transfer cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [final_fields[&0]]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [final_fields[&1]]);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() == final_root));
}

#[test]
fn nested_partial_struct_transfer_preserves_recursive_topology_and_mask() {
    let (source, raw) = nested_owned_partial_local_transfer_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested partial transfer");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct transfer");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let target_root = roots[&2];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("nested partial whole-root move");
    let temporary = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::Temporary(value) if Some(value) == whole_move.result()
            )
        })
        .expect("nested partial transfer temporary")
        .id();
    let topology = |root| {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        let inner = fields[&0];
        let tail = fields[&1];
        let inner_fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == inner => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(inner_fields.keys().copied().collect::<Vec<_>>(), [0]);
        let text = inner_fields[&0];
        (inner, text, tail)
    };
    let source_topology = topology(source_root);
    let temporary_topology = topology(temporary);
    let target_topology = topology(target_root);
    assert_ne!(source_topology, temporary_topology);
    assert_ne!(temporary_topology, target_topology);
    let cleanup_actions = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    let cleanup = cleanup_actions
        .iter()
        .find(|action| action.root() == target_root)
        .expect("nested transferred-root cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [target_topology.1]);
    assert_eq!(cleanup.initialized_projections().collect::<Vec<_>>(), [target_topology.2]);
    assert!(
        cleanup_actions
            .iter()
            .all(|action| action.root() != source_root && action.root() != temporary)
    );
}

#[test]
fn nested_partial_struct_return_preserves_recursive_topology_and_reverse_survivors() {
    let (source, raw) = nested_owned_partial_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested partial return");
    let program = lower(pair_input(&syntax, &sources)).expect("nested partial Struct return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let transferred_root = roots[&2];
    let survivor_root = roots[&3];
    let block = function.blocks().next().expect("block");
    let returned = block.terminator().value_operands().next().expect("returned value");
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
        })
        .expect("nested partial return temporary")
        .id();
    let topology = |root| {
        let fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == root => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
        let inner_fields = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::StructField { base, ordinal } if base == fields[&0] => {
                    Some((ordinal, place.id()))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(inner_fields.keys().copied().collect::<Vec<_>>(), [0]);
        (inner_fields[&0], fields[&1])
    };
    let source_topology = topology(source_root);
    let transferred_topology = topology(transferred_root);
    let returned_topology = topology(temporary);
    assert_ne!(source_topology, transferred_topology);
    assert_ne!(transferred_topology, returned_topology);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        cleanup
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [survivor_root, moved_leaf_owner,]
    );
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != transferred_root
            && action.root() != temporary
            && action.root() != returned_topology.0
    }));
}
#[test]
fn partial_transfer_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_transfer_place_delta(0, 0), Some(2));
    assert_eq!(partial_transfer_place_delta(2, 0), Some(8));
    assert_eq!(partial_transfer_place_delta(2, 1), Some(7));
    assert_eq!(partial_transfer_place_delta(2, 2), Some(6));
    assert_eq!(partial_transfer_place_delta(1, 2), None);
    assert_eq!(partial_transfer_place_delta(usize::MAX, 0), None);

    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 2, 0, 2, 1),
        Ok(7),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values, places - 7, transitions - 2, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 6, transitions - 2, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_transfer_budget_preflight(values - 1, places - 7, transitions - 2, 1, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_transfer_budget_preflight(0, 0, 0, 0, usize::MAX, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
    assert_eq!(
        partial_transfer_budget_preflight(usize::MAX, 0, 0, 0, 0, 0),
        Err(PartialTransferBudgetViolation::Values),
    );
}
#[test]
fn partial_return_place_accounting_is_exact_and_checked() {
    assert_eq!(partial_return_place_delta(0, 0), Some(1));
    assert_eq!(partial_return_place_delta(2, 0), Some(5));
    assert_eq!(partial_return_place_delta(2, 1), Some(4));
    assert_eq!(partial_return_place_delta(2, 2), Some(3));
    assert_eq!(partial_return_place_delta(1, 2), None);
    assert_eq!(partial_return_place_delta(usize::MAX, 0), None);

    let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let places = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;
    let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions - 1, 0, 2, 1),
        Ok(4),
    );
    assert_eq!(
        partial_return_budget_preflight(values, places - 4, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Values),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 3, transitions - 1, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Places),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions, 0, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_return_budget_preflight(values - 1, places - 4, transitions - 1, 1, 2, 1),
        Err(PartialTransferBudgetViolation::Transitions),
    );
    assert_eq!(
        partial_return_budget_preflight(0, 0, 0, 0, usize::MAX, 0),
        Err(PartialTransferBudgetViolation::PlaceAccounting),
    );
    assert_eq!(
        partial_return_budget_preflight(usize::MAX, 0, 0, 0, 0, 0),
        Err(PartialTransferBudgetViolation::Values),
    );
}
#[test]
fn partial_struct_owner_returns_with_exact_topology_mask_and_survivor_cleanup() {
    let (source, raw) = owned_pair_partial_then_root_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful partial Struct return");
    let program = lower(pair_input(&syntax, &sources)).expect("partial Struct return");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_leaf_owner = roots[&1];
    let block = function.blocks().next().expect("block");
    let whole_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(source_root)
        })
        .expect("partial root return move");
    let returned = block.terminator().value_operands().next().expect("returned value");
    assert_eq!(whole_move.result(), Some(returned));
    let temporary = function
        .places()
        .find(|place| {
            matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == returned)
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
    let source_fields = fields(source_root);
    let returned_fields = fields(temporary);
    assert_eq!(source_fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(returned_fields.keys().copied().collect::<Vec<_>>(), [0, 1]);
    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].root(), moved_leaf_owner);
    assert!(cleanup.iter().all(|action| {
        action.root() != source_root
            && action.root() != temporary
            && action.root() != returned_fields[&0]
    }));
}
