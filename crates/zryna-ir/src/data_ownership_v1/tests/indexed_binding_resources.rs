use super::indexed_access_resources::at_capacity;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn bind_at_capacity(fixture: &Fixture, count: usize) -> raw::Program {
    let mut program = at_capacity(fixture, count);
    let function = &mut program.modules[0].functions[0];
    let parent = u32::try_from(count).expect("bounded parent");
    function.blocks[0].instructions.insert(
        count + 1,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::BindIndexedBorrow {
                parent: raw::BorrowId(parent),
                borrow: raw::BorrowId(parent + 1),
            },
        },
    );
    function.blocks[0].instructions[count + 2].kind =
        raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(parent + 1) };
    program
}

#[test]
fn indexed_binding_preserves_active_count_and_allocates_no_cleanup_or_place() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let verified = fixture.verify(bind_at_capacity(&fixture, 8));
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.places().count(), 3);
    let block = function.blocks().next().expect("block");
    assert_eq!(block.instructions().count(), 18);
    let instruction = block.instructions().nth(9).expect("bind");
    assert_eq!(instruction.indexed_binding().expect("binding").borrow().index(), 9);
    assert_eq!(instruction.failure_ended_borrows().count(), 0);
}

#[test]
#[ignore = "full bound lexical active-authority exact/first-extra boundary"]
fn indexed_binding_active_exact_first_extra_and_recovery() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    fixture.verify(bind_at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION));
    fixture.rejects(bind_at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION + 1), "ZRYNA-I3201");
    fixture.verify(bind_at_capacity(&fixture, 1));
}
