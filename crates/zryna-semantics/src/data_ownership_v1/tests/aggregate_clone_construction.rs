use super::*;

#[test]
fn private_owned_struct_prepares_in_source_order_and_commits_in_declaration_order() {
    let sources = sources_for(OWNED_PAIR_SOURCE);
    let syntax = verify_snapshot(response_snapshot(OWNED_PAIR_RESPONSE), &sources)
        .expect("source-faithful owned Pair v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned Pair must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::BoolLiteral,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    assert_eq!(
        instructions[2]
            .value_operands()
            .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            .collect::<Vec<_>>(),
        vec![1, 0],
        "constructor operands follow declaration order after source-order preparation",
    );
    assert_eq!(instructions[1].derived_drop_actions().count(), 0);
    assert_eq!(instructions[2].cleanup(), None);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn string_bearing_struct_clone_retains_source_and_seals_recursive_prefix_cleanup() {
    let (source, raw) = clone_final_return_snapshot(OWNED_PAIR_SOURCE, OWNED_PAIR_RESPONSE);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful aggregate clone v4");
    let program = lower(pair_input(&syntax, &sources)).expect("owned aggregate clone must verify");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        vec![
            VerifiedInstructionKind::BoolLiteral,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::ClonePlace,
        ]
    );
    let clone = instructions[4];
    let source_owner = clone.place_operands().next().expect("source owner");
    let result_owner = function
        .places()
        .find(|place| matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if Some(value) == clone.result()))
        .expect("distinct clone result owner")
        .id();
    assert_ne!(source_owner, result_owner);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![source_owner],
    );
    let failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(failure.len(), 2);
    assert_eq!(failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(failure[0].root(), result_owner);
    assert_eq!(failure[1].kind(), VerifiedDropActionKind::Place);
    assert_eq!(failure[1].root(), source_owner);
    assert_eq!(
        block.terminator().derived_drop_actions().map(|action| action.root()).collect::<Vec<_>>(),
        vec![source_owner],
        "successful return transfers only the distinct clone and retains the source",
    );
}

#[test]
fn string_bearing_fixed_array_and_enum_clone_use_the_same_recursive_failure_authority() {
    for (source, response, label, leaf_count, active_variant) in [
        (OWNED_ARRAY_SOURCE, OWNED_ARRAY_RESPONSE, "fixed array", 2, None),
        (OWNED_ENUM_STRING_SOURCE, OWNED_ENUM_STRING_RESPONSE, "active enum variant", 1, Some(1)),
    ] {
        let (source, raw) = clone_final_return_snapshot(source, response);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("source-faithful structural clone v4");
        let program = lower(pair_input(&syntax, &sources)).expect(label);
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let clone = block
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
            .expect("structural clone");
        let source_owner = clone.place_operands().next().expect("source owner");
        let result_owner = function
            .places()
            .find(|place| {
                matches!(place.kind(), VerifiedPlaceKind::Temporary(value) if Some(value) == clone.result())
            })
            .expect("clone owner")
            .id();
        let failure = clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(leaf_count),);
        assert_eq!(failure[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
        assert_eq!(failure[0].root(), result_owner);
        assert_eq!(failure[0].active_variant(), active_variant);
        assert_eq!(
            failure[0]
                .active_variants()
                .find(|variant| variant.place() == result_owner)
                .map(VerifiedActiveVariant::variant),
            active_variant,
        );
        assert!(failure.iter().skip(1).any(|action| action.root() == source_owner));
        assert!(
            block.terminator().derived_drop_actions().any(|action| action.root() == source_owner)
        );
    }
}
