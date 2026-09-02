use super::*;

#[test]
fn authenticated_input_rejects_another_source_authority() {
    let sources = pair_sources();
    let raw = decode_snapshot(PAIR_JSON).expect("Pair v4 JSON");
    let syntax = verify_snapshot(raw, &sources).expect("Pair v4 authority");
    let other = pair_sources();
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = other.file_id(&path).expect("entry");
    assert!(SemanticInput::try_new(&syntax, &other, entry).is_none());
}

#[test]
fn private_multibyte_string_literal_has_distinct_prepare_and_return_cleanup() {
    let sources = sources_for(MULTIBYTE_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(MULTIBYTE_STRING_RESPONSE), &sources)
        .expect("source-faithful multibyte String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String literal");
    assert_eq!(
        program.runtime_abi().type_universe_identity(),
        program.verified_ir().type_universe_identity()
    );
    let function = program.modules().next().expect("module").functions().next().expect("function");
    assert!(function.public_export().is_none());
    let block = function.blocks().next().expect("block");
    let instruction = block.instructions().next().expect("StringFromUtf8");
    assert_eq!(instruction.kind(), VerifiedInstructionKind::StringFromUtf8);
    assert_eq!(instruction.string_utf8_bytes(), Some("snowman: ☃".as_bytes()));
    assert_eq!(instruction.cleanup().expect("prepare cleanup").index(), 0);
    assert_eq!(instruction.derived_drop_actions().count(), 0);
    let terminator = block.terminator();
    assert_eq!(terminator.kind(), VerifiedTerminatorKind::Return);
    assert_eq!(terminator.cleanup().expect("return cleanup").index(), 1);
    assert_eq!(terminator.derived_drop_actions().count(), 0);
}

#[test]
fn private_string_local_moves_exact_owner_to_return() {
    let sources = sources_for(LOCAL_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(LOCAL_STRING_RESPONSE), &sources)
        .expect("source-faithful local String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String local");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
        .collect::<Vec<_>>();
    assert_eq!(
        instructions,
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::MoveFromPlace,
        ]
    );
    let places =
        function.places().map(zryna_ir::data_ownership_v1::VerifiedPlace::kind).collect::<Vec<_>>();
    assert!(matches!(places[0], VerifiedPlaceKind::Temporary(value) if value.index() == 0));
    assert_eq!(places[1], VerifiedPlaceKind::Local(0));
    assert!(matches!(places[2], VerifiedPlaceKind::Temporary(value) if value.index() == 1));
    let block = function.blocks().next().expect("block");
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
}

#[test]
fn private_string_return_cleanup_drops_remaining_locals_in_reverse_order() {
    let sources = sources_for(THREE_LOCAL_STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(THREE_LOCAL_STRING_RESPONSE), &sources)
        .expect("source-faithful three-local String v4");
    let program = lower(pair_input(&syntax, &sources)).expect("three private String locals");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let prepare_roots = block
        .instructions()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::StringFromUtf8)
        .map(|instruction| {
            instruction
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(prepare_roots, [vec![], vec![1], vec![3, 1]]);
    let roots = block
        .terminator()
        .derived_drop_actions()
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(roots, [3, 1]);
}

#[test]
fn private_string_use_after_move_is_rejected_deterministically() {
    let sources = sources_for(USE_AFTER_MOVE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(USE_AFTER_MOVE_RESPONSE), &sources)
        .expect("source-faithful moved String v4");
    let first = lower(pair_input(&syntax, &sources)).expect_err("use after move");
    let second = lower(pair_input(&syntax, &sources)).expect_err("same use after move");
    let summary = |diagnostics: &[zryna_diagnostics::Diagnostic]| {
        diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code().to_owned(), diagnostic.message().to_owned()))
            .collect::<Vec<_>>()
    };
    let diagnostic = first
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("semantic use-after-move diagnostic");
    let at = diagnostic.primary_span().expect("reference span");
    assert_eq!((at.start(), at.end()), (89, 94));
    assert_eq!(summary(&first), summary(&second));
}

#[test]
fn private_string_clone_retains_source_at_prepare_and_return() {
    let sources = sources_for(STRING_CLONE_SOURCE);
    let raw = response_snapshot(STRING_CLONE_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 2);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String clone v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String clone");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringClone)
        .expect("StringClone");
    let source = clone.place_operands().next().expect("clone source");
    assert_eq!(source.index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1]
    );
}

#[test]
fn private_string_concat_retains_both_sources_in_reverse_cleanup_order() {
    let sources = sources_for(STRING_CONCAT_SOURCE);
    let raw = response_snapshot(STRING_CONCAT_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 3);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String concat call v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let concat = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::StringConcat)
        .expect("StringConcat");
    assert_eq!(
        concat
            .place_operands()
            .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    let expected = [3, 1];
    assert_eq!(
        concat.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn private_string_concat_full_single_block_shape_is_stable() {
    let sources = sources_for(STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful String concat v4");
    let program = lower(pair_input(&syntax, &sources)).expect("private String concat");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id().index(), 0);
    let instructions = blocks[0].instructions().collect::<Vec<_>>();
    assert_eq!(
        instructions.iter().map(|instruction| instruction.kind()).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::StringConcat,
        ]
    );
    assert_eq!(
        instructions
            .iter()
            .filter_map(|instruction| {
                instruction.result().map(zryna_ir::data_ownership_v1::ValueIdentity::index)
            })
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    let plans = function
        .cleanup_plans()
        .map(|plan| {
            (
                plan.id().index(),
                plan.site().role(),
                plan.actions()
                    .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plans,
        [
            (0, VerifiedCleanupRole::PrepareFailure, vec![]),
            (1, VerifiedCleanupRole::PrepareFailure, vec![1]),
            (2, VerifiedCleanupRole::PrepareFailure, vec![3, 1]),
            (3, VerifiedCleanupRole::Return, vec![3, 1]),
        ]
    );
    let terminator = blocks[0].terminator();
    let return_start =
        u32::try_from(STRING_CONCAT_SOURCE.find("return").expect("return")).expect("return offset");
    assert_eq!(terminator.span().start(), return_start);
    assert_eq!(terminator.span().end(), return_start + 27);
    assert_eq!(terminator.value_operands().next().expect("return value").index(), 2);
    assert_eq!(terminator.cleanup().expect("return cleanup").index(), 3);
}

#[test]
fn private_string_clone_rejects_a_moved_source_at_its_reference() {
    let sources = sources_for(MOVED_STRING_CLONE_SOURCE);
    let syntax = verify_snapshot(response_snapshot(MOVED_STRING_CLONE_RESPONSE), &sources)
        .expect("source-faithful moved String clone v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("clone after move");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3011")
        .expect("moved source diagnostic");
    let at = diagnostic.primary_span().expect("source reference");
    assert_eq!((at.start(), at.end()), (96, 102));
}

#[test]
fn private_string_concat_requires_exact_builtin_arity() {
    let sources = sources_for(BAD_STRING_CONCAT_SOURCE);
    let syntax = verify_snapshot(response_snapshot(BAD_STRING_CONCAT_RESPONSE), &sources)
        .expect("source-faithful malformed concat call v4");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("concat arity");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-M3012")
        .expect("String concat diagnostic");
    let at = diagnostic.primary_span().expect("concat callee");
    assert_eq!((at.start(), at.end()), (60, 66));
}
