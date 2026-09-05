use super::super::PlaceIdentity;
use super::generic_clone_fixture::Fixture;
use super::*;
use zryna_layout::TypeCategory;

fn refined(fixture: &Fixture) -> raw::Program {
    let mut program = fixture.seed();
    let enumeration =
        fixture.linear.types().find(|ty| ty.category() == TypeCategory::Enum).expect("enum");
    let enum_ty = raw::TypeId(enumeration.id().index());
    let inner = raw::TypeId(enumeration.variants()[1].payload().expect("inner").index());
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.result = enum_ty;
    function.places[2].ty = inner;
    let kinds = [
        (enum_ty, raw::PlaceKind::Temporary(raw::ValueId(4))),
        (enum_ty, raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 }),
        (inner, raw::PlaceKind::EnumPayload { base: raw::PlaceId(4), variant: 1 }),
        (fixture.integer, raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 1 }),
        (fixture.string, raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 }),
        (enum_ty, raw::PlaceKind::Temporary(raw::ValueId(5))),
        (inner, raw::PlaceKind::EnumPayload { base: raw::PlaceId(8), variant: 1 }),
        (fixture.integer, raw::PlaceKind::StructField { base: raw::PlaceId(9), ordinal: 1 }),
        (fixture.string, raw::PlaceKind::StructField { base: raw::PlaceId(9), ordinal: 0 }),
        (fixture.string, raw::PlaceKind::EnumPayload { base: raw::PlaceId(8), variant: 0 }),
    ];
    for (ty, kind) in kinds {
        function.places.push(raw::Place {
            id: raw::PlaceId(u32::try_from(function.places.len()).expect("small fixture")),
            ty,
            span,
            kind,
        });
    }
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(value(3, inner)),
            span,
            kind: raw::InstructionKind::StructConstruct {
                fields: vec![raw::ValueId(1), raw::ValueId(2)],
                cleanup: None,
            },
        },
        raw::Instruction {
            result: Some(value(4, enum_ty)),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(3)),
                cleanup: None,
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::GenericReplacePlace {
                place: raw::PlaceId(4),
                value: raw::ValueId(4),
            },
        },
        raw::Instruction {
            result: Some(value(5, enum_ty)),
            span,
            kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) },
        },
        raw::Instruction {
            result: Some(value(6, fixture.integer)),
            span,
            kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(10) },
        },
    ];
    function.cleanup_plans = vec![raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span,
        actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(0) };
    program
}

#[test]
fn generic_static_refinement_moves_known_enum_and_invalidates_source_descendants() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let seed = refined(&fixture);
    let verified = fixture.verify(seed.clone());
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    assert_eq!(
        block.instructions().nth(4).expect("payload read").kind(),
        VerifiedInstructionKind::CopyFromPlace
    );
    let cleanup = block.terminator().derived_drop_actions().next().expect("retained root");
    assert_eq!(cleanup.root().index(), 0);
    assert_eq!(
        cleanup.moved_projections().map(PlaceIdentity::index).collect::<Vec<_>>(),
        [4, 5, 6, 7]
    );
    let mut old_source = seed;
    old_source.modules[0].functions[0].blocks[0].instructions[4].kind =
        raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(6) };
    fixture.rejects(old_source, "ZRYNA-I3010");
}

#[test]
fn generic_static_refinement_does_not_invent_an_unknown_source_tag() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let mut program = refined(&fixture);
    // Discard the prepared known-tag value instead of installing it in the source.
    program.modules[0].functions[0].blocks[0].instructions[2].kind =
        raw::InstructionKind::DropPlace { place: raw::PlaceId(3) };
    fixture.rejects(program, "ZRYNA-I3013");
}

#[test]
fn generic_static_refinement_rejects_the_other_destination_variant() {
    let fixture = Fixture::new(TypeCategory::Struct);
    let mut program = refined(&fixture);
    fixture.verify(program.clone());
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(13),
        ty: fixture.string,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(7)),
    });
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(7), ty: fixture.string, span }),
        span,
        kind: raw::InstructionKind::StringClone {
            place: raw::PlaceId(12),
            cleanup: raw::CleanupPlanId(1),
        },
    });
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![
            raw::DropAction::DropPlace(raw::PlaceId(8)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
    });
    function.cleanup_plans[0].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(13)));
    fixture.rejects(program, "ZRYNA-I3013");
}
