use super::indexed_access::chained;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn bound(fixture: &Fixture, access: raw::BorrowAccess) -> raw::Program {
    let mut program = chained(fixture, access);
    let function = &mut program.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        2,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::BindIndexedBorrow {
                parent: raw::BorrowId(1),
                borrow: raw::BorrowId(2),
            },
        },
    );
    function.blocks[0].instructions[3].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(2) };
    program
}

#[test]
fn indexed_binding_seals_lexical_type_region_and_infallible_parent_transfer() {
    for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
        let fixture = Fixture::new(Container::Array, Element::Array);
        let program = bound(&fixture, access);
        let index =
            super::super::BorrowIndex::new(&program.modules[0].functions[0], &fixture.linear);
        assert!(index.origin(raw::BorrowId(2)).is_none());
        assert_eq!(index.definition(raw::BorrowId(1)), index.definition(raw::BorrowId(2)));
        assert_eq!(index.region(raw::BorrowId(1)), index.region(raw::BorrowId(2)));
        let verified = fixture.verify(program);
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let binding = function.blocks().next().expect("block").instructions().nth(2).expect("bind");
        let view = binding.indexed_binding().expect("sealed binding");
        assert_eq!(view.parent().index(), 1);
        assert_eq!(view.borrow().index(), 2);
        assert_eq!(view.container().index(), 0);
        assert_eq!(view.access(), access.into());
        assert!(binding.result().is_none());
        assert_eq!(binding.derived_drop_actions().count(), 0);
        assert_eq!(binding.failure_ended_borrows().count(), 0);
    }
}

#[test]
fn indexed_binding_rejects_rebinding_and_retired_or_non_dense_parents() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut rebound = bound(&fixture, raw::BorrowAccess::Shared);
    let function = &mut rebound.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        3,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::BindIndexedBorrow {
                parent: raw::BorrowId(2),
                borrow: raw::BorrowId(3),
            },
        },
    );
    fixture.rejects(rebound, "ZRYNA-I3005");
    let mut retired = bound(&fixture, raw::BorrowAccess::Shared);
    let function = &mut retired.modules[0].functions[0];
    function.blocks[0].instructions.insert(2, end_borrow(1, function.span));
    fixture.rejects(retired, "ZRYNA-I3011");
    let mut sparse = bound(&fixture, raw::BorrowAccess::Shared);
    sparse.modules[0].functions[0].blocks[0].instructions[2].kind =
        raw::InstructionKind::BindIndexedBorrow {
            parent: raw::BorrowId(1),
            borrow: raw::BorrowId(3),
        };
    fixture.rejects(sparse, "ZRYNA-I3011");
    fixture.verify(bound(&fixture, raw::BorrowAccess::Shared));
}

#[test]
fn indexed_binding_rejects_lexical_parent_and_result_value() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut lexical = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut lexical.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::BindIndexedBorrow {
                parent: raw::BorrowId(0),
                borrow: raw::BorrowId(1),
            },
        },
    );
    function.blocks[0].instructions[2].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(1) };
    fixture.rejects(lexical, "ZRYNA-I3005");
    let mut result = bound(&fixture, raw::BorrowAccess::Shared);
    let function = &mut result.modules[0].functions[0];
    function.blocks[0].instructions[2].result = Some(raw::ValueDefinition {
        id: raw::ValueId(5),
        ty: fixture.integer,
        span: function.span,
    });
    fixture.rejects(result, "ZRYNA-I3005");
}
