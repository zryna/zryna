use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

pub(super) fn at_capacity(fixture: &Fixture, count: usize) -> raw::Program {
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let actions = function.cleanup_plans[0].actions.clone();
    function.cleanup_plans = (0..count + 2)
        .map(|id| raw::CleanupPlan {
            id: raw::CleanupPlanId(u32::try_from(id).expect("bounded cleanup")),
            span,
            actions: actions.clone(),
        })
        .collect();
    let instructions = &mut function.blocks[0].instructions;
    instructions.clear();
    for id in 0..count {
        let id = u32::try_from(id).expect("bounded borrow");
        instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginIndexedAccess {
                definition: raw::BorrowDefinition {
                    id: raw::BorrowId(id),
                    place: raw::PlaceId(0),
                    access: raw::BorrowAccess::Shared,
                    span,
                },
                index: raw::ValueId(2),
                cleanup: raw::CleanupPlanId(id),
            },
        });
    }
    let child = u32::try_from(count).expect("bounded child");
    instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::ProjectIndexedBorrow {
            parent: raw::BorrowId(child - 1),
            borrow: raw::BorrowId(child),
            index: raw::ValueId(3),
            cleanup: raw::CleanupPlanId(child),
        },
    });
    instructions.push(end_borrow(child, span));
    for id in (0..child - 1).rev() {
        instructions.push(end_borrow(id, span));
    }
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(child + 1) };
    program
}

#[test]
fn indexed_access_projection_balances_resources_without_element_places() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    let verified = fixture.verify(at_capacity(&fixture, 8));
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.places().count(), 3);
    let block = function.blocks().next().expect("block");
    let projection = block.instructions().nth(8).expect("projection");
    assert_eq!(
        projection.failure_ended_borrows().map(|id| id.index()).collect::<Vec<_>>(),
        (0..8).rev().collect::<Vec<_>>()
    );
    assert_eq!(block.instructions().count(), 17);
}

#[test]
#[ignore = "full transient active-authority exact/first-extra boundary"]
fn indexed_access_active_exact_projection_first_extra_and_recovery() {
    let fixture = Fixture::new(Container::Array, Element::Array);
    fixture.verify(at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION));
    fixture.rejects(at_capacity(&fixture, MAX_ACTIVE_BORROWS_PER_FUNCTION + 1), "ZRYNA-I3201");
    fixture.verify(at_capacity(&fixture, 1));
}
