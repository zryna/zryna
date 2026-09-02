use super::*;

#[test]
fn projected_aggregate_assignment_moves_one_complete_root_into_a_static_field() {
    let sources = sources_for(PROJECTED_AGGREGATE_ASSIGNMENT_SOURCE);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_AGGREGATE_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected aggregate assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected aggregate assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected aggregate replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(moved.place_operands().next(), Some(source));
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("static target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = moved.result().and_then(|value| {
        function.places().find(
            |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
        )
    });
    assert!(temporary.is_some(), "whole-root move has one distinct temporary owner");
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the old projected aggregate subtree",
    );
    assert_eq!(instructions[replace_index + 1].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(instructions[replace_index + 1].place_operands().next(), Some(destination));
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn projected_aggregate_assignment_clones_one_complete_root_into_a_static_field() {
    let (source, raw) = projected_aggregate_clone_assignment_snapshot();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful clone assignment");
    let program =
        lower(pair_input(&syntax, &sources)).expect("projected aggregate clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected aggregate replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    assert_eq!(clone.place_operands().next(), Some(source));
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("static target");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
        "clone allocation failure retains source and destination",
    );
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        element_failure[1..]
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [source, destination],
        "prefix failure retains both pre-existing roots",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit drops only the old projected subtree",
    );
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source],
        "successful clone retains the source root until function exit",
    );
}

#[test]
fn projected_aggregate_assignment_moves_one_static_subobject_between_distinct_roots() {
    let source = PROJECTED_SUBOBJECT_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected subobject assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected subobject assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    let source_projection = moved.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source projection place")
            .kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == source
    ));
    let source_leaf = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 }
                    if base == source_projection
            )
        })
        .expect("complete source descendant topology")
        .id();
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target subtree",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source]
    );
    assert_eq!(
        exit[0].moved_projections().collect::<Vec<_>>(),
        [source_projection, source_leaf],
        "source parent remains pending with the complete moved subtree masked",
    );
}

#[test]
fn projected_aggregate_assignment_moves_one_fixed_array_element_between_distinct_roots() {
    let source = FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(FIXED_ARRAY_SUBOBJECT_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful fixed-array subobject assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("fixed-array subobject assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("fixed-array projected replacement");
    let moved = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
    let source_projection = moved.place_operands().next().expect("source array projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source array projection place")
            .kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 1 } if base == source
    ));
    let source_leaf = function
        .places()
        .find(|place| {
            matches!(
                place.kind(),
                VerifiedPlaceKind::StructField { base, ordinal: 0 }
                    if base == source_projection
            )
        })
        .expect("complete source array element topology")
        .id();
    assert_eq!(replace.value_operands().next(), moved.result());
    let target = replace.place_operands().next().expect("target array projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == destination
    ));
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target array element",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source]
    );
    assert_eq!(
        exit[0].moved_projections().collect::<Vec<_>>(),
        [source_projection, source_leaf],
        "source array remains pending with the moved element subtree masked",
    );
}

#[test]
fn projected_aggregate_assignment_clones_one_struct_subobject_between_distinct_roots() {
    let source = PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax =
        verify_snapshot(response_snapshot(PROJECTED_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE), &sources)
            .expect("source-faithful projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("projected clone assignment");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("projected clone replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    let source_projection = clone.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source projection place")
            .kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == source
    ));
    assert!(
        !function.places().any(|place| matches!(
            place.kind(),
            VerifiedPlaceKind::StructField { base, .. } if base == source_projection
        )),
        "projected clone materializes only the canonical source path",
    );
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::StructField { base, ordinal: 0 } if base == destination
    ));
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
        "prepare failure retains source and destination roots",
    );
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        element_failure[1..]
            .iter()
            .map(zryna_ir::data_ownership_v1::VerifiedDropAction::root)
            .collect::<Vec<_>>(),
        [source, destination],
        "prefix failure retains both pre-existing roots",
    );
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
        "commit recursively drops only the old target subtree",
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source],
    );
    assert_eq!(
        exit[0].moved_projections().count(),
        0,
        "successful clone retains the complete source subtree",
    );
}

#[test]
fn projected_aggregate_assignment_clones_one_fixed_array_subobject_between_distinct_roots() {
    let source = FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_SOURCE.trim_end();
    let sources = sources_for(source);
    let syntax = verify_snapshot(
        response_snapshot(FIXED_ARRAY_SUBOBJECT_CLONE_ASSIGNMENT_RESPONSE),
        &sources,
    )
    .expect("source-faithful fixed-array projected clone assignment");
    let program = lower(pair_input(&syntax, &sources)).expect("fixed-array projected clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let roots = function
        .places()
        .filter_map(|place| match place.kind() {
            VerifiedPlaceKind::Local(ordinal) => Some((ordinal, place.id())),
            _ => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let destination = roots[&0];
    let source = roots[&1];
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    let replace_index = instructions
        .iter()
        .position(|instruction| instruction.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("fixed-array projected clone replacement");
    let clone = instructions[replace_index - 1];
    let replace = instructions[replace_index];
    assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
    let source_projection = clone.place_operands().next().expect("source projection");
    assert!(matches!(
        function
            .places()
            .find(|place| place.id() == source_projection)
            .expect("source place")
            .kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 1 } if base == source
    ));
    assert!(
        !function.places().any(|place| matches!(
            place.kind(),
            VerifiedPlaceKind::StructField { base, .. } if base == source_projection
        )),
        "fixed-array clone does not materialize descendant places",
    );
    assert_eq!(replace.value_operands().next(), clone.result());
    let target = replace.place_operands().next().expect("target projection");
    assert!(matches!(
        function.places().find(|place| place.id() == target).expect("target place").kind(),
        VerifiedPlaceKind::FixedArrayConstant { base, index: 0 } if base == destination
    ));
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [source, destination],
    );
    let temporary = clone
        .result()
        .and_then(|value| {
            function.places().find(
                |place| matches!(place.kind(), VerifiedPlaceKind::Temporary(owner) if owner == value),
            )
        })
        .expect("clone temporary")
        .id();
    let element_failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(element_failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(element_failure[0].root(), temporary);
    assert_eq!(
        replace.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        [target],
    );
    let exit = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(
        exit.iter().map(zryna_ir::data_ownership_v1::VerifiedDropAction::root).collect::<Vec<_>>(),
        [source],
    );
    assert_eq!(exit[0].moved_projections().count(), 0);
}
