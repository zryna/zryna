use super::indexed_borrow_fixture::{Container, ELEMENTS, Element, Fixture};
use super::*;
use crate::data_ownership_v1::{VerifiedBorrowAccess, VerifiedTrapIdentity};

#[test]
fn indexed_borrow_zero_length_and_zero_sized_fixed_arrays_keep_runtime_bounds() {
    for (element, length, nested_length) in [(Element::I32, 0, 2), (Element::Array, 2, 0)] {
        let fixture = Fixture::with_lengths(Container::Array, element, length, nested_length);
        for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            let verified = fixture.verify(fixture.seed(access));
            let begin = verified
                .modules()
                .next()
                .expect("module")
                .functions()
                .next()
                .expect("function")
                .blocks()
                .next()
                .expect("block")
                .instructions()
                .next()
                .expect("begin");
            let authority = begin.indexed_borrow().expect("indexed authority");
            assert_eq!(authority.array_length(), Some(length));
            assert_eq!(authority.referent().index(), fixture.element.0);
            assert_eq!(authority.trap_identity(), VerifiedTrapIdentity::BoundsV1);
        }
    }
}

#[test]
fn indexed_borrow_generic_referents_seal_container_index_type_bounds_and_retained_owners() {
    for container in [Container::Array, Container::Vec] {
        for element in ELEMENTS {
            let fixture = Fixture::new(container, element);
            for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
                let raw = fixture.seed(access);
                let expected = raw.modules[0].functions[0].cleanup_plans[0].actions.clone();
                let mut prior = None;
                for _ in 0..2 {
                    let verified = fixture.verify(raw.clone());
                    let function = verified
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("function");
                    let block = function.blocks().next().expect("block");
                    let instruction = block.instructions().next().expect("begin");
                    let authority = instruction.indexed_borrow().expect("indexed authority");
                    assert_eq!(authority.borrow().index(), 0);
                    assert_eq!(authority.container().index(), 0);
                    assert_eq!(authority.index().index(), 2);
                    assert_eq!(authority.referent().index(), fixture.element.0);
                    assert_eq!(
                        authority.access(),
                        if access == raw::BorrowAccess::Shared {
                            VerifiedBorrowAccess::Shared
                        } else {
                            VerifiedBorrowAccess::Exclusive
                        }
                    );
                    assert_eq!(authority.cleanup().index(), 0);
                    assert_eq!(authority.trap_identity(), VerifiedTrapIdentity::BoundsV1);
                    assert_eq!(instruction.failure_ended_borrows().count(), 0);
                    assert_eq!(
                        authority.array_length(),
                        if matches!(container, Container::Array) { Some(2) } else { None }
                    );
                    assert!(
                        instruction.result().is_none(),
                        "creation cannot produce an owned element"
                    );
                    let failed = instruction.derived_drop_actions().collect::<Vec<_>>();
                    let returned = block.terminator().derived_drop_actions().collect::<Vec<_>>();
                    let roots = expected
                        .iter()
                        .map(|action| match action {
                            raw::DropAction::DropPlace(place) => place.0,
                            _ => panic!("complete owners only"),
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        failed.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
                        roots
                    );
                    assert_eq!(failed, returned, "begin/end retains every owner and mask");
                    assert!(returned.iter().all(|action| action.moved_projections().count() == 0));
                    if let Some(previous) = prior.replace((failed.clone(), returned.clone())) {
                        assert_eq!(
                            (failed, returned),
                            previous,
                            "sealed replay {container:?}/{element:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn indexed_borrow_copy_read_write_and_runtime_invalid_indices_remain_typed() {
    for container in [Container::Array, Container::Vec] {
        for element in [Element::Bool, Element::I32] {
            let fixture = Fixture::new(container, element);
            let mut seed = fixture.seed(raw::BorrowAccess::Exclusive);
            let function = &mut seed.modules[0].functions[0];
            let span = function.span;
            function.blocks[0].instructions.insert(
                1,
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(5),
                        ty: fixture.element,
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
                },
            );
            function.blocks[0].instructions.insert(
                2,
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::BorrowWrite {
                        borrow: raw::BorrowId(0),
                        value: raw::ValueId(5),
                    },
                },
            );
            fixture.verify(seed.clone());
            for value in [-1, 0, 1, 2, i32::MAX] {
                let mut raw = seed.clone();
                let function = &mut raw.modules[0].functions[0];
                function.blocks[0].instructions.insert(
                    0,
                    raw::Instruction {
                        result: Some(raw::ValueDefinition {
                            id: raw::ValueId(5),
                            ty: fixture.integer,
                            span,
                        }),
                        span,
                        kind: raw::InstructionKind::I32Literal(value),
                    },
                );
                let raw::InstructionKind::BeginIndexedBorrow { index, .. } =
                    &mut function.blocks[0].instructions[1].kind
                else {
                    panic!("begin")
                };
                *index = raw::ValueId(5);
                function.blocks[0].instructions[2].result.as_mut().expect("read").id =
                    raw::ValueId(6);
                let raw::InstructionKind::BorrowWrite { value, .. } =
                    &mut function.blocks[0].instructions[3].kind
                else {
                    panic!("write")
                };
                *value = raw::ValueId(6);
                let verified = fixture.verify(raw);
                let instruction = verified
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function")
                    .blocks()
                    .next()
                    .expect("block")
                    .instructions()
                    .nth(1)
                    .expect("begin");
                assert_eq!(
                    instruction.indexed_borrow().expect("authority").trap_identity(),
                    VerifiedTrapIdentity::BoundsV1
                );
            }
        }
    }
}
