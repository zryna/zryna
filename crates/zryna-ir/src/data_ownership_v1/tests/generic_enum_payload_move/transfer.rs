use super::*;

#[test]
fn generic_enum_payload_move_old_enum_drop_precedes_complete_result_edge_transfer() {
    let fixture = Fixture::new(TypeCategory::Enum);
    let mut raw = seed(&fixture, false);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[2].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    function.blocks[2].terminators[0].kind = raw::Terminator::Jump(raw::Edge {
        target: raw::BlockId(3),
        arguments: vec![raw::ValueId(4)],
    });
    function.places.push(raw::Place {
        id: raw::PlaceId(8),
        ty: function.result,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
    });
    function.blocks.push(raw::Block {
        id: raw::BlockId(3),
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(5), ty: function.result, span }],
        instructions: vec![],
        terminators: vec![raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Return {
                value: raw::ValueId(5),
                cleanup: raw::CleanupPlanId(1),
            },
        }],
    });
    function.cleanup_plans[1].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(1))];
    for _ in 0..2 {
        let verified = check(&fixture, raw.clone())
            .expect("complete owned edge transfer after old enum cleanup");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let blocks = function.blocks().collect::<Vec<_>>();
        let drop = blocks[2]
            .instructions()
            .nth(1)
            .expect("explicit old enum drop")
            .derived_drop_actions()
            .next()
            .expect("old root obligation");
        assert_eq!(drop.root().index(), 0);
        assert_eq!(
            drop.moved_projections()
                .map(super::super::super::PlaceIdentity::index)
                .collect::<Vec<_>>(),
            [4, 5, 6]
        );
        assert_eq!(
            drop.active_variants()
                .map(|variant| (variant.place().index(), variant.variant()))
                .collect::<Vec<_>>(),
            [(0, 1)]
        );
        assert_eq!(
            blocks[2]
                .terminator()
                .edges()
                .next()
                .expect("owned edge")
                .arguments()
                .map(super::super::super::ValueIdentity::index)
                .collect::<Vec<_>>(),
            [4]
        );
        assert_eq!(blocks[3].parameters().next().expect("owned block parameter").id().index(), 5);
        assert_eq!(
            blocks[3]
                .terminator()
                .derived_drop_actions()
                .map(|drop| drop.root().index())
                .collect::<Vec<_>>(),
            [1]
        );
    }
    let mut duplicate = raw.clone();
    duplicate.modules[0].functions[0].cleanup_plans[1]
        .actions
        .push(raw::DropAction::DropPlace(raw::PlaceId(0)));
    reject(&fixture, &raw, duplicate, "ZRYNA-I3012");
}
