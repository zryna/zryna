use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn refined(fixture: &Fixture) -> raw::Program {
    let mut raw = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(3),
            ty: fixture.wrapper,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: fixture.root,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 1 },
        },
    ]);
    if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
        &mut function.blocks[0].instructions[0].kind
    {
        definition.place = raw::PlaceId(4);
    }
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.wrapper, span }),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            },
        },
    );
    for plan in &mut function.cleanup_plans {
        plan.actions = vec![
            raw::DropAction::DropPlace(raw::PlaceId(3)),
            raw::DropAction::DropPlace(raw::PlaceId(2)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
        ];
    }
    raw
}

#[test]
fn indexed_borrow_authenticates_active_container_payload_and_preserves_refinement() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::String);
        let raw = refined(&fixture);
        let verified = fixture.verify(raw.clone());
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let begin = block.instructions().nth(1).expect("indexed begin");
        assert_eq!(begin.indexed_borrow().expect("authority").container().index(), 4);
        let before = begin.derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(before[0].root().index(), 3);
        assert_eq!(before[0].active_variant(), Some(1));
        assert_eq!(before, block.terminator().derived_drop_actions().collect::<Vec<_>>());
        let mut inactive = raw;
        inactive.modules[0].functions[0].places[4].kind =
            raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 0 };
        fixture.rejects(inactive, "ZRYNA-I3013");
    }
}

#[test]
fn indexed_borrow_container_payload_conflicts_with_enclosing_owner_in_both_orders() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::String);
        let seed = refined(&fixture);
        fixture.verify(seed.clone());
        for root_first in [false, true] {
            let mut raw = seed.clone();
            let function = &mut raw.modules[0].functions[0];
            let span = function.span;
            if root_first {
                if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
                    &mut function.blocks[0].instructions[1].kind
                {
                    definition.id = raw::BorrowId(1);
                }
                function.blocks[0]
                    .instructions
                    .insert(1, begin_borrow(0, 3, raw::BorrowAccess::Exclusive, span));
                function.blocks[0].instructions.insert(3, end_borrow(1, span));
            } else {
                function.blocks[0]
                    .instructions
                    .insert(2, begin_borrow(1, 3, raw::BorrowAccess::Exclusive, span));
                function.blocks[0].instructions.insert(3, end_borrow(1, span));
            }
            fixture.rejects(raw, "ZRYNA-I3011");
        }
    }
}
