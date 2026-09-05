use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn copy_temporary(fixture: &Fixture) -> raw::Program {
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.root,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
    });
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.root, span }),
            span,
            kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(0) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(5),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginIndexedAccess {
                definition: raw::BorrowDefinition {
                    id: raw::BorrowId(0),
                    place: raw::PlaceId(3),
                    access: raw::BorrowAccess::Shared,
                    span,
                },
                index: raw::ValueId(2),
                cleanup: raw::CleanupPlanId(0),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(6), ty: fixture.integer, span }),
            span,
            kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
        },
        end_borrow(0, span),
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(6), cleanup: raw::CleanupPlanId(1) };
    program
}

#[test]
fn indexed_access_copy_storage_requires_real_initialization_without_owned_cleanup() {
    let fixture = Fixture::new(Container::Array, Element::I32);
    let raw = copy_temporary(&fixture);
    let verified = fixture.verify(raw.clone());
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let instructions = block.instructions().collect::<Vec<_>>();
    assert_eq!(instructions[1].kind(), VerifiedInstructionKind::InitializePlace);
    assert_eq!(instructions[2].indexed_borrow().expect("bounds").container().index(), 3);
    assert_eq!(instructions[2].derived_drop_actions().count(), 0);
    assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    let mut missing = raw;
    missing.modules[0].functions[0].blocks[0].instructions.remove(1);
    fixture.rejects(missing, "ZRYNA-I3011");
    fixture.verify(copy_temporary(&fixture));
}
