use super::*;

#[test]
fn generic_enum_payload_move_rejects_wrong_variant_and_absent_refinement() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let valid = seed(&fixture, false);
    check(&fixture, valid.clone()).expect("baseline");
    let mut wrong = valid.clone();
    if let raw::Terminator::EnumMatch { arms, .. } =
        &mut wrong.modules[0].functions[0].blocks[0].terminators[0].kind
    {
        arms[0].edge.target = raw::BlockId(2);
        arms[1].edge.target = raw::BlockId(1);
    }
    reject(&fixture, &valid, wrong, "ZRYNA-I3013");
    let mut unrefined = valid.clone();
    let function = &mut unrefined.modules[0].functions[0];
    function.blocks.truncate(1);
    function.places[2].kind = raw::PlaceKind::Temporary(raw::ValueId(3));
    function.places.remove(3);
    for (id, place) in function.places.iter_mut().enumerate() {
        place.id = raw::PlaceId(u32::try_from(id).expect("small places"));
        if let raw::PlaceKind::StructField { base, .. } = &mut place.kind {
            *base = raw::PlaceId(3);
        }
    }
    function.blocks[0].instructions.push(raw::Instruction {
        span: function.span,
        result: Some(raw::ValueDefinition {
            id: raw::ValueId(3),
            ty: function.result,
            span: function.span,
        }),
        kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(3) },
    });
    function.cleanup_plans.truncate(1);
    function.cleanup_plans[0].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(1)));
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(0) };
    reject(&fixture, &valid, unrefined, "ZRYNA-I3013");
}

#[test]
fn generic_enum_payload_move_rejects_repeated_borrowed_moved_and_partial_sources() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let valid = seed(&fixture, false);
    for mutation in 0..5 {
        let mut forged = valid.clone();
        let function = &mut forged.modules[0].functions[0];
        let span = function.span;
        match mutation {
            0 => {
                function.places.push(raw::Place {
                    id: raw::PlaceId(8),
                    ty: function.result,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
                });
                function.blocks[2].instructions.push(raw::Instruction {
                    span,
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(5),
                        ty: function.result,
                        span,
                    }),
                    kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) },
                });
                function.cleanup_plans[1]
                    .actions
                    .insert(0, raw::DropAction::DropPlace(raw::PlaceId(8)));
            }
            1 | 2 => {
                function.blocks[2].instructions.insert(
                    0,
                    begin_borrow(
                        0,
                        if mutation == 1 { 0 } else { 4 },
                        raw::BorrowAccess::Shared,
                        span,
                    ),
                );
                function.blocks[2].instructions.push(end_borrow(0, span));
            }
            3 => function.blocks[2].instructions.insert(
                0,
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
                },
            ),
            4 => {
                function.places[3].kind = raw::PlaceKind::Temporary(raw::ValueId(5));
                function.blocks[2].instructions[0].result.as_mut().expect("result").id =
                    raw::ValueId(5);
                if let raw::Terminator::Return { value, .. } =
                    &mut function.blocks[2].terminators[0].kind
                {
                    *value = raw::ValueId(5);
                }
                function.places.push(raw::Place {
                    id: raw::PlaceId(8),
                    ty: fixture.string,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
                });
                function.blocks[2].instructions.insert(
                    0,
                    raw::Instruction {
                        span,
                        result: Some(raw::ValueDefinition {
                            id: raw::ValueId(4),
                            ty: fixture.string,
                            span,
                        }),
                        kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(5) },
                    },
                );
                function.cleanup_plans[1]
                    .actions
                    .insert(0, raw::DropAction::DropPlace(raw::PlaceId(8)));
            }
            _ => unreachable!(),
        }
        reject(&fixture, &valid, forged, "ZRYNA-I3010");
    }
}

#[test]
fn generic_enum_payload_move_rejects_forged_paths_types_and_cross_arm_result_use() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let valid = seed(&fixture, false);
    for mutation in 0..4 {
        let mut forged = valid.clone();
        let function = &mut forged.modules[0].functions[0];
        let code = match mutation {
            0 => {
                function.places[4].kind =
                    raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 99 };
                "ZRYNA-I3006"
            }
            1 => {
                function.places[4].kind =
                    raw::PlaceKind::EnumPayload { base: raw::PlaceId(4), variant: 1 };
                "ZRYNA-I3006"
            }
            2 => {
                function.places[3].ty = fixture.string;
                function.blocks[2].instructions[0].result.as_mut().expect("result").ty =
                    fixture.string;
                "ZRYNA-I3005"
            }
            3 => {
                if let raw::Terminator::Return { value, .. } =
                    &mut function.blocks[1].terminators[0].kind
                {
                    *value = raw::ValueId(4);
                }
                "ZRYNA-I3008"
            }
            _ => unreachable!(),
        };
        reject(&fixture, &valid, forged, code);
    }
}

#[test]
fn generic_enum_payload_move_cleanup_is_exact_ordered_and_excludes_transferred_owner() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let valid = seed(&fixture, false);
    for mutation in 0..4 {
        let mut forged = valid.clone();
        let actions = &mut forged.modules[0].functions[0].cleanup_plans[1].actions;
        match mutation {
            0 => actions.clear(),
            1 => actions.reverse(),
            2 => actions.push(raw::DropAction::DropPlace(raw::PlaceId(0))),
            3 => actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(3))),
            _ => unreachable!(),
        }
        let errors = check(&fixture, forged.clone()).expect_err("invalid cleanup");
        assert_eq!(
            diagnostic_trace(errors),
            vec![(
                "ZRYNA-I3012".into(),
                if mutation == 2 {
                    "cleanup plan has a noncanonical identity or foreign place".into()
                } else {
                    "cleanup plan is incomplete, duplicated, or out of reverse-completion order"
                        .into()
                },
                Some((
                    valid.modules[0].functions[0].span.start(),
                    valid.modules[0].functions[0].span.end()
                ))
            )]
        );
        reject(&fixture, &valid, forged, "ZRYNA-I3012");
    }
}
