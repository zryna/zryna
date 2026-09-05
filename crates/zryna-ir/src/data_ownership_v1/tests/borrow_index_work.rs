use super::super::borrow_index::BorrowIndex;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

#[test]
fn early_lexical_borrow_many_reads_share_one_dense_index() {
    let fixture = Fixture::new(Container::Array, Element::I32);
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let reads = MAX_VALUES_PER_FUNCTION - function.parameters.len();
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
            id: raw::BorrowId(0),
            place: raw::PlaceId(2),
            access: raw::BorrowAccess::Shared,
            span,
        }),
    }];
    for offset in 0..reads {
        function.blocks[0].instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(u32::try_from(offset + 5).expect("bounded value")),
                ty: fixture.integer,
                span,
            }),
            span,
            kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
        });
    }
    function.blocks[0].instructions.push(end_borrow(0, span));
    function.cleanup_plans.truncate(1);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    let verified = fixture.verify(program);
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let index = function.borrows();
    assert_eq!(index.construction_steps, reads + 2);
    assert_eq!(index.parent_steps, 0);
    for _ in 0..reads {
        assert_eq!(
            index.definition(raw::BorrowId(0)),
            Some((fixture.integer, raw::BorrowAccess::Shared))
        );
        assert_eq!(index.region(raw::BorrowId(0)), Some(raw::PlaceId(2)));
        assert!(index.origin(raw::BorrowId(0)).is_none());
    }
}

#[test]
fn hostile_transient_parent_chain_has_linear_index_work() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let count = MAX_ACTIVE_BORROWS_PER_FUNCTION;
    function.blocks[0].instructions.clear();
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::BeginIndexedAccess {
            definition: raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(0),
                access: raw::BorrowAccess::Shared,
                span,
            },
            index: raw::ValueId(2),
            cleanup: raw::CleanupPlanId(0),
        },
    });
    for ordinal in 1..count {
        let id = u32::try_from(ordinal).expect("bounded borrow");
        function.blocks[0].instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ProjectIndexedBorrow {
                parent: raw::BorrowId(id - 1),
                borrow: raw::BorrowId(id),
                index: raw::ValueId(3),
                cleanup: raw::CleanupPlanId(1),
            },
        });
    }
    // The hostile chain exceeds the actual array nesting. Index construction must
    // remain linear before type validation rejects it; this is not a verified program.
    let index = BorrowIndex::new(function, &fixture.linear);
    drop(program);
    assert_eq!(index.construction_steps, count);
    assert_eq!(index.parent_steps, count - 1);
    for ordinal in 1..count {
        let id = raw::BorrowId(u32::try_from(ordinal).expect("bounded borrow"));
        assert_eq!(index.origin(id).map(|(_, depth)| depth), Some(ordinal));
        assert_eq!(index.region(id), Some(raw::PlaceId(0)));
        if ordinal > 1 {
            assert!(index.definition(id).is_none());
        }
    }
}
