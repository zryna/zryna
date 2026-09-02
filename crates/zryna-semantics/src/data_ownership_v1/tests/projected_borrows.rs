use super::*;

#[test]
fn projected_borrows_preserve_exact_static_paths_and_disjoint_authority() {
    let sources = sources_for(PROJECTED_BORROW_SOURCE);
    let raw = decode_snapshot(PROJECTED_BORROW_JSON).expect("projected-borrow snapshot");
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 19);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful projected-borrow v4");
    let program = lower(pair_input(&syntax, &sources)).expect("projected-borrow lowering");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let places = function.places().collect::<Vec<_>>();
    assert_eq!(places.len(), 14);
    assert!(places.iter().enumerate().all(|(index, place)| {
        place.id().index() == u32::try_from(index).expect("bounded place index")
    }));
    let topology = places
        .iter()
        .map(|place| match place.kind() {
            VerifiedPlaceKind::Local(index) => ("local", index, 0),
            VerifiedPlaceKind::StructField { base, ordinal } => ("field", base.index(), ordinal),
            VerifiedPlaceKind::FixedArrayConstant { base, index } => ("array", base.index(), index),
            kind => panic!("unexpected projected-borrow place: {kind:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        topology,
        vec![
            ("local", 0, 0),
            ("field", 0, 0),
            ("field", 1, 0),
            ("field", 0, 1),
            ("field", 3, 0),
            ("local", 5, 0),
            ("field", 1, 1),
            ("local", 7, 0),
            ("field", 0, 2),
            ("array", 8, 0),
            ("local", 10, 0),
            ("local", 11, 0),
            ("array", 8, 1),
            ("local", 13, 0),
        ]
    );

    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let begins = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .map(|instruction| {
            let operands = instruction.place_operands().collect::<Vec<_>>();
            assert_eq!(operands.len(), 1);
            (
                operands[0].index(),
                instruction.borrow().expect("borrow identity").index(),
                instruction.borrow_access().expect("borrow access"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        begins,
        vec![
            (2, 0, VerifiedBorrowAccess::Exclusive),
            (4, 1, VerifiedBorrowAccess::Exclusive),
            (9, 2, VerifiedBorrowAccess::Shared),
            (9, 3, VerifiedBorrowAccess::Shared),
            (12, 4, VerifiedBorrowAccess::Exclusive),
        ]
    );
    let ended = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .map(|instruction| instruction.borrow().expect("ended borrow").index())
        .collect::<Vec<_>>();
    assert_eq!(ended, vec![4, 3, 2, 1, 0]);
    assert_eq!(function.borrow_parameters().count(), 0);
    assert_eq!(function.cleanup_plans().next().expect("return cleanup").actions().count(), 0);
}

#[test]
fn projected_borrow_lowering_replays_the_complete_place_and_authority_trace() {
    let lower_trace = || {
        let sources = sources_for(PROJECTED_BORROW_SOURCE);
        let syntax = verify_snapshot(
            decode_snapshot(PROJECTED_BORROW_JSON).expect("projected-borrow replay snapshot"),
            &sources,
        )
        .expect("source-faithful projected-borrow replay v4");
        let program = lower(pair_input(&syntax, &sources)).expect("projected-borrow replay");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let places = function
            .places()
            .map(|place| match place.kind() {
                VerifiedPlaceKind::Local(index) => ("local", index, 0),
                VerifiedPlaceKind::StructField { base, ordinal } => {
                    ("field", base.index(), ordinal)
                }
                VerifiedPlaceKind::FixedArrayConstant { base, index } => {
                    ("array", base.index(), index)
                }
                kind => panic!("unexpected replay place: {kind:?}"),
            })
            .collect::<Vec<_>>();
        let instructions = function
            .blocks()
            .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
            .map(|instruction| {
                (
                    instruction.kind(),
                    instruction.borrow().map(zryna_ir::data_ownership_v1::BorrowIdentity::index),
                    instruction.borrow_access(),
                    instruction
                        .place_operands()
                        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        (places, instructions)
    };
    assert_eq!(lower_trace(), lower_trace());
}

#[test]
fn projected_borrow_resource_formula_is_exact_saturating_and_preflighted() {
    let resources = projected_root_borrow_resource_counts(10, 5, 5, 3, 3, 8);
    assert_eq!(resources.values, 19);
    assert_eq!(resources.places, 14);
    assert_eq!(resources.transitions, 38);
    assert_eq!(resources.blocks, 1);
    assert_eq!(resources.edges, 0);
    assert_eq!(resources.active_peak, 5);
    assert_eq!(resources.cleanup_plans, 1);
    assert_eq!(root_borrow_resource_violation(resources), None);

    for (exact, first_extra, expected) in [
        (
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
                ..RootBorrowResources::default()
            },
            RootBorrowResources {
                values: zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION + 1,
                ..RootBorrowResources::default()
            },
            RootBorrowBudgetLimit::Values,
        ),
        (
            RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
                ..RootBorrowResources::default()
            },
            RootBorrowResources {
                places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION + 1,
                ..RootBorrowResources::default()
            },
            RootBorrowBudgetLimit::Places,
        ),
        (
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                ..RootBorrowResources::default()
            },
            RootBorrowResources {
                transitions: zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    + 1,
                ..RootBorrowResources::default()
            },
            RootBorrowBudgetLimit::Transitions,
        ),
        (
            RootBorrowResources {
                active_peak: zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION,
                ..RootBorrowResources::default()
            },
            RootBorrowResources {
                active_peak: zryna_ir::data_ownership_v1::MAX_ACTIVE_BORROWS_PER_FUNCTION + 1,
                ..RootBorrowResources::default()
            },
            RootBorrowBudgetLimit::ActiveBorrows,
        ),
    ] {
        assert_eq!(root_borrow_resource_violation(exact), None);
        assert_eq!(root_borrow_resource_violation(first_extra), Some(expected));
    }

    let saturated = projected_root_borrow_resource_counts(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    );
    assert_eq!(saturated.values, usize::MAX);
    assert_eq!(saturated.places, usize::MAX);
    assert_eq!(saturated.transitions, usize::MAX);
    assert_eq!(saturated.active_peak, usize::MAX);
    assert_eq!(root_borrow_resource_violation(saturated), Some(RootBorrowBudgetLimit::Values));
}

#[test]
fn overlapping_shared_parent_and_child_keep_independent_verified_authority() {
    let sources = sources_for(PROJECTED_BORROW_SHARED_OVERLAP_SOURCE);
    let syntax = verify_snapshot(
        decode_snapshot(PROJECTED_BORROW_SHARED_OVERLAP_JSON)
            .expect("shared projected overlap snapshot"),
        &sources,
    )
    .expect("source-faithful shared projected overlap v4");
    let program = lower(pair_input(&syntax, &sources)).expect("overlapping shared borrows");
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let begins = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::BeginBorrow)
        .map(|instruction| {
            (
                instruction.place_operands().next().expect("borrow place").index(),
                instruction.borrow().expect("borrow identity").index(),
                instruction.borrow_access().expect("borrow access"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        begins,
        vec![(0, 0, VerifiedBorrowAccess::Shared), (1, 1, VerifiedBorrowAccess::Shared),]
    );
    let ended = instructions
        .iter()
        .filter(|instruction| instruction.kind() == VerifiedInstructionKind::EndBorrow)
        .map(|instruction| instruction.borrow().expect("ended borrow").index())
        .collect::<Vec<_>>();
    assert_eq!(ended, vec![1, 0]);
}

#[test]
fn projected_borrow_exclusions_are_exact_ordered_and_deterministic() {
    let lower_trace = || {
        let sources = sources_for(PROJECTED_BORROW_EXCLUSIONS_SOURCE);
        let syntax = verify_snapshot(
            decode_snapshot(PROJECTED_BORROW_EXCLUSIONS_JSON)
                .expect("projected-borrow exclusions snapshot"),
            &sources,
        )
        .expect("source-faithful projected-borrow exclusions v4");
        lower(pair_input(&syntax, &sources))
            .expect_err("projected-borrow exclusions")
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_owned(),
                    diagnostic.message().to_owned(),
                    diagnostic.primary_span().map(|span| (span.start(), span.end())),
                )
            })
            .collect::<Vec<_>>()
    };
    let first = lower_trace();
    assert_eq!(first, lower_trace());
    assert_eq!(first.len(), 13, "ordered trace: {first:#?}");
    let expected_messages = [
        "borrow access conflicts with an active alias of an overlapping place",
        "borrow access conflicts with an active alias of an overlapping place",
        "borrow access conflicts with an active alias of an overlapping place",
        "owner reads are hidden by an overlapping exclusive alias",
        "struct 'State' has no borrowable field 'missing'",
        "borrow fixed-array index 2 is outside length 2",
        "borrow fixed-array index is negative or outside u32",
        "dynamic fixed-array borrowing conservatively overlaps the complete root and is unavailable",
        "root borrowing requires an exact recursively Copy result",
        "root borrowing requires an exact recursively Copy result",
        "root borrowing requires an exact recursively Copy result",
        "borrow field projection does not have a Struct base",
        "borrow access conflicts with an active alias of an overlapping place",
    ];
    assert_eq!(
        first.iter().map(|(code, _, _)| code.as_str()).collect::<Vec<_>>(),
        vec!["ZRYNA-M3017"; 13]
    );
    assert_eq!(
        first.iter().map(|(_, message, _)| message.as_str()).collect::<Vec<_>>(),
        expected_messages
    );
    assert_eq!(
        first.iter().map(|(_, _, primary)| *primary).collect::<Vec<_>>(),
        vec![
            Some((477, 535)),
            Some((766, 813)),
            Some((1_046, 1_093)),
            Some((1_391, 1_402)),
            Some((1_611, 1_618)),
            Some((1_867, 1_868)),
            Some((2_116, 2_118)),
            Some((2_365, 2_376)),
            Some((2_432, 2_609)),
            Some((2_611, 2_782)),
            Some((2_784, 2_999)),
            Some((3_183, 3_202)),
            Some((3_475, 3_528)),
        ]
    );
}
