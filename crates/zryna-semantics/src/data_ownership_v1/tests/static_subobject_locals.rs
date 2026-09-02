use super::*;

#[test]
fn static_struct_subobject_moves_into_one_exact_direct_local() {
    let sources = sources_for(PROJECTED_INNER_MOVE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(PROJECTED_INNER_MOVE_RESPONSE), &sources)
        .expect("source-faithful projected Inner move");
    let program = lower(pair_input(&syntax, &sources)).expect("projected Inner move");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_local = roots[&1];
    let source_fields = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal } if base == source_root => {
                Some((ordinal, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(source_fields.keys().copied().collect::<Vec<_>>(), [0]);
    let moved_inner = source_fields[&0];
    let moved_text = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal }
                if base == moved_inner && ordinal == 0 =>
            {
                Some(place.id())
            }
            _ => None,
        })
        .expect("moved Inner text projection");
    let block = function.blocks().next().expect("block");
    let projection_move = block
        .instructions()
        .find(|instruction| {
            instruction.kind() == VerifiedInstructionKind::MoveFromPlace
                && instruction.place_operands().next() == Some(moved_inner)
        })
        .expect("projected aggregate move");
    let moved_value = projection_move.result().expect("projected move result");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::InitializePlace
            && instruction.place_operands().next() == Some(moved_local)
            && instruction.value_operands().next() == Some(moved_value)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == source_root)
        .expect("partial source cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved_inner, moved_text]);
    assert_eq!(cleanup.initialized_projections().count(), 0);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() != moved_local));
}

#[test]
fn static_fixed_array_subobject_move_preserves_the_disjoint_element() {
    let sources = sources_for(PROJECTED_ARRAY_ELEMENT_MOVE_SOURCE);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_ARRAY_ELEMENT_MOVE_RESPONSE), &sources)
            .expect("source-faithful projected fixed-array element move");
    let program = lower(pair_input(&syntax, &sources)).expect("projected fixed-array move");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let source_root = roots[&0];
    let moved_local = roots[&1];
    let elements = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::FixedArrayConstant { base, index } if base == source_root => {
                Some((index, place.id()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(elements.keys().copied().collect::<Vec<_>>(), [0]);
    let block = function.blocks().next().expect("block");
    let moved_element = elements[&0];
    let moved_text = function
        .places()
        .find_map(|place| match place.kind() {
            VerifiedPlaceKind::StructField { base, ordinal }
                if base == moved_element && ordinal == 0 =>
            {
                Some(place.id())
            }
            _ => None,
        })
        .expect("moved array element text projection");
    assert!(block.instructions().any(|instruction| {
        instruction.kind() == VerifiedInstructionKind::MoveFromPlace
            && instruction.place_operands().next() == Some(moved_element)
    }));
    let cleanup = block
        .terminator()
        .derived_drop_actions()
        .find(|action| action.root() == source_root)
        .expect("partial array cleanup");
    assert_eq!(cleanup.moved_projections().collect::<Vec<_>>(), [moved_element, moved_text]);
    assert_eq!(cleanup.initialized_projections().count(), 0);
    assert!(block.terminator().derived_drop_actions().all(|action| action.root() != moved_local));
}

#[test]
#[allow(clippy::too_many_lines)]
fn static_projected_aggregate_clone_initializes_one_exact_local_and_retains_source() {
    for (source, response, label) in [
        (PROJECTED_INNER_MOVE_SOURCE, PROJECTED_INNER_MOVE_RESPONSE, "StructField"),
        (
            PROJECTED_ARRAY_ELEMENT_MOVE_SOURCE,
            PROJECTED_ARRAY_ELEMENT_MOVE_RESPONSE,
            "FixedArrayConstant",
        ),
    ] {
        let (source, raw) = projected_aggregate_clone_local_snapshot(source, response);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected clone");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let abi = program.runtime_abi();
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let roots = function
            .places()
            .filter_map(|place| match place.kind() {
                VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let source_root = roots[&0];
        let cloned_local = roots[&1];
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let clone_index = instructions
            .iter()
            .position(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
            .expect("projected aggregate clone");
        let clone = instructions[clone_index];
        let projected = clone.place_operands().next().expect("projected source");
        let source_kind = function
            .places()
            .find(|place| place.id() == projected)
            .expect("projected place")
            .kind();
        assert!(
            matches!(
                (label, source_kind),
                ("StructField", VerifiedPlaceKind::StructField { base, ordinal: 0 })
                    if base == source_root
            ) || matches!(
                (label, source_kind),
                (
                    "FixedArrayConstant",
                    VerifiedPlaceKind::FixedArrayConstant { base, index: 0 }
                ) if base == source_root
            )
        );
        let result = clone.result().expect("clone result");
        let temporary = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if value == result)
            })
            .expect("clone temporary")
            .id();
        let initialize = instructions.get(clone_index + 1).expect("immediate local initialization");
        assert_eq!(initialize.kind(), VerifiedInstructionKind::InitializePlace);
        assert_eq!(initialize.place_operands().next(), Some(cloned_local));
        assert_eq!(initialize.value_operands().next(), Some(result));
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(1));
        assert_eq!(
            clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
            [source_root],
        );
        let element_failure =
            clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
        assert_eq!(element_failure[0].root(), temporary);
        assert_eq!(element_failure[1].kind(), VerifiedDropActionKind::Place);
        assert_eq!(element_failure[1].root(), source_root);
        let first = owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 0,
            },
            0,
            1,
        )
        .expect("projected aggregate clone fault");
        let replay = owned_fault_trace(
            abi,
            function,
            clone,
            OwnedFaultInjection::AggregateCloneElement {
                status: RuntimeStatus::Allocation,
                completed_prefix: 0,
            },
            0,
            1,
        )
        .expect("deterministic projected aggregate clone fault");
        assert_eq!(first, replay);
        assert!(!first.result_committed);
        assert_eq!(first.prefix_owner, Some(temporary));
        assert_eq!(first.retained_roots, [source_root]);
        assert_eq!(first.reverse_cleanup, [source_root]);
    }
}

#[test]
fn projected_aggregate_clone_stays_excluded_from_direct_return() {
    let (source, raw) = projected_aggregate_clone_direct_return_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful direct projected clone");
    let diagnostics =
        lower(pair_input(&syntax, &sources)).expect_err("direct projected clone return");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3013");
    assert_eq!(
        diagnostics[0].primary_span(),
        Some(span(&sources, nth_untrusted_span(&source, "o.inner", 0),)),
    );
}
