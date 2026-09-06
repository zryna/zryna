use super::generic_clone_fixture::Fixture;
use super::*;
use zryna_layout::TypeCategory;

mod hostile;
mod resources;
mod transfer;

fn seed(fixture: &Fixture, nested: bool) -> raw::Program {
    let mut raw = fixture.seed();
    let enumeration =
        fixture.linear.types().find(|ty| ty.category() == TypeCategory::Enum).expect("enum");
    let enum_ty = raw::TypeId(enumeration.id().index());
    let inner = raw::TypeId(enumeration.variants()[1].payload().expect("owned struct").index());
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.result = inner;
    function.places[2].ty = inner;
    let enum_place = if nested { 3 } else { 0 };
    for (ty, kind) in [
        (inner, raw::PlaceKind::Temporary(raw::ValueId(4))),
        (inner, raw::PlaceKind::EnumPayload { base: raw::PlaceId(enum_place), variant: 1 }),
        (fixture.string, raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 0 }),
        (fixture.integer, raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 1 }),
        (
            fixture.string,
            raw::PlaceKind::EnumPayload { base: raw::PlaceId(enum_place), variant: 0 },
        ),
    ] {
        function.places.push(raw::Place {
            id: raw::PlaceId(u32::try_from(function.places.len()).expect("small places")),
            ty,
            span,
            kind,
        });
    }
    if nested {
        function.places.push(raw::Place {
            id: raw::PlaceId(8),
            ty: enum_ty,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        });
        function.places.swap(3, 8);
        function.places[3].id = raw::PlaceId(3);
        function.places[8].id = raw::PlaceId(8);
    }
    function.blocks[0].instructions.clear();
    function.blocks[0].terminators[0].kind = raw::Terminator::EnumMatch {
        place: raw::PlaceId(enum_place),
        arms: (0..2)
            .map(|variant| raw::EnumArm {
                variant,
                edge: raw::Edge { target: raw::BlockId(variant + 1), arguments: vec![] },
            })
            .collect(),
    };
    for (block, kind, value_id, cleanup) in [
        (
            1,
            raw::InstructionKind::StructConstruct {
                fields: vec![raw::ValueId(1), raw::ValueId(2)],
                cleanup: None,
            },
            3,
            0,
        ),
        (2, raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) }, 4, 1),
    ] {
        function.blocks.push(raw::Block {
            id: raw::BlockId(block),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(value(value_id, inner)),
                span,
                kind,
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(value_id),
                    cleanup: raw::CleanupPlanId(cleanup),
                },
            }],
        });
    }
    function.cleanup_plans = [vec![0], vec![1, 0]]
        .into_iter()
        .enumerate()
        .map(|(id, places)| raw::CleanupPlan {
            id: raw::CleanupPlanId(u32::try_from(id).expect("two plans")),
            span,
            actions: places
                .into_iter()
                .map(|id| raw::DropAction::DropPlace(raw::PlaceId(id)))
                .collect(),
        })
        .collect();
    raw
}

fn check(
    fixture: &Fixture,
    raw: raw::Program,
) -> Result<super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
    verify(
        raw,
        &fixture.sources,
        fixture.sources.verify_file_id(0).expect("entry"),
        fixture.linear.clone(),
        fixture.linux.clone(),
    )
}

fn reject(fixture: &Fixture, valid: &raw::Program, forged: raw::Program, code: &str) {
    let errors = check(fixture, forged.clone()).expect_err("forged enum move");
    assert!(errors.iter().any(|error| error.code() == code), "expected {code}: {errors:?}");
    assert_eq!(errors, check(fixture, forged).expect_err("deterministic rejection"));
    check(fixture, valid.clone()).expect("authenticated recovery after rejection");
}

#[test]
fn generic_enum_payload_move_transfers_complete_owned_result_and_masks_only_active_subtree() {
    for nested in [false, true] {
        let fixture = Fixture::new(if nested { TypeCategory::Struct } else { TypeCategory::Enum });
        let raw = seed(&fixture, nested);
        let mut traces = vec![];
        for _ in 0..2 {
            let verified =
                check(&fixture, raw.clone()).expect("refined complete owned payload move");
            let function =
                verified.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().nth(2).expect("variant one arm");
            let instruction = block.instructions().next().expect("move");
            assert_eq!(instruction.kind(), VerifiedInstructionKind::GenericMoveFromPlace);
            assert_eq!(instruction.result().expect("owned result").index(), 4);
            assert_eq!(instruction.place_operands().next().expect("exact source").index(), 4);
            assert_eq!(
                instruction.result_type().expect("exact payload").index(),
                raw.modules[0].functions[0].result.0
            );
            assert_eq!(
                instruction.derived_drop_actions().len(),
                0,
                "move is infallible and does not drop source"
            );
            let drops = block.terminator().derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(drops.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(), [1, 0]);
            assert_eq!(
                drops[1]
                    .moved_projections()
                    .map(super::super::PlaceIdentity::index)
                    .collect::<Vec<_>>(),
                [4, 5, 6]
            );
            assert_eq!(
                drops[1]
                    .active_variants()
                    .map(|variant| (variant.place().index(), variant.variant()))
                    .collect::<Vec<_>>(),
                [(if nested { 3 } else { 0 }, 1)]
            );
            assert!(
                !drops.iter().any(|drop| drop.root().index() == if nested { 8 } else { 3 }),
                "returned complete payload owner is transferred, not dropped"
            );
            traces.push(format!("{drops:?}"));
        }
        assert_eq!(traces[0], traces[1]);
    }
}
