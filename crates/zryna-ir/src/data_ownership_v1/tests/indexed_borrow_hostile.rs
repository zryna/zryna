use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

#[test]
fn indexed_borrow_rejects_wrong_container_index_type_undefined_and_late_values() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::Bool);
        let seed = fixture.seed(raw::BorrowAccess::Shared);
        fixture.verify(seed.clone());
        let mut boolean_index = seed.clone();
        let function = &mut boolean_index.modules[0].functions[0];
        function.parameters[3].ty = fixture.boolean;
        if let raw::InstructionKind::BeginIndexedBorrow { index, .. } =
            &mut function.blocks[0].instructions[0].kind
        {
            *index = raw::ValueId(3);
        }
        fixture.rejects(boolean_index, "ZRYNA-I3005");
        for (index, code) in [(4, "ZRYNA-I3005"), (99, "ZRYNA-I3008")] {
            let mut raw = seed.clone();
            if let raw::InstructionKind::BeginIndexedBorrow { index: value, .. } =
                &mut raw.modules[0].functions[0].blocks[0].instructions[0].kind
            {
                *value = raw::ValueId(index);
            }
            fixture.rejects(raw, code);
        }
        let mut scalar_container = seed.clone();
        if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
            &mut scalar_container.modules[0].functions[0].blocks[0].instructions[0].kind
        {
            definition.place = raw::PlaceId(2);
        }
        fixture.rejects(scalar_container, "ZRYNA-I3005");
        let mut late = seed;
        let function = &mut late.modules[0].functions[0];
        if let raw::InstructionKind::BeginIndexedBorrow { index, .. } =
            &mut function.blocks[0].instructions[0].kind
        {
            *index = raw::ValueId(5);
        }
        function.blocks[0].instructions.insert(
            1,
            raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(5),
                    ty: fixture.integer,
                    span: function.span,
                }),
                span: function.span,
                kind: raw::InstructionKind::I32Literal(0),
            },
        );
        fixture.rejects(late, "ZRYNA-I3008");
    }
}

#[test]
fn indexed_borrow_rejects_sparse_duplicate_identity_and_inactive_access() {
    let fixture = Fixture::new(Container::Vec, Element::I32);
    let seed = fixture.seed(raw::BorrowAccess::Shared);
    fixture.verify(seed.clone());
    let mut sparse = seed.clone();
    if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
        &mut sparse.modules[0].functions[0].blocks[0].instructions[0].kind
    {
        definition.id = raw::BorrowId(3);
    }
    fixture.rejects(sparse, "ZRYNA-I3011");
    let mut duplicate = seed.clone();
    let function = &mut duplicate.modules[0].functions[0];
    let mut begin = function.blocks[0].instructions[0].clone();
    if let raw::InstructionKind::BeginIndexedBorrow { cleanup, .. } = &mut begin.kind {
        *cleanup = raw::CleanupPlanId(2);
    }
    let mut plan = function.cleanup_plans[0].clone();
    plan.id = raw::CleanupPlanId(2);
    function.cleanup_plans.push(plan);
    function.blocks[0].instructions.insert(1, begin);
    fixture.rejects(duplicate, "ZRYNA-I3011");
    let mut inactive = seed;
    let function = &mut inactive.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition {
            id: raw::ValueId(5),
            ty: fixture.element,
            span: function.span,
        }),
        span: function.span,
        kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
    });
    fixture.rejects(inactive, "ZRYNA-I3011");
}

#[test]
fn indexed_borrow_bounds_cleanup_rejects_missing_duplicate_reordered_and_reused_claims() {
    let fixture = Fixture::new(Container::Vec, Element::String);
    let seed = fixture.seed(raw::BorrowAccess::Shared);
    fixture.verify(seed.clone());
    for damage in 0..4 {
        let mut raw = seed.clone();
        let function = &mut raw.modules[0].functions[0];
        match damage {
            0 => {
                function.cleanup_plans[0].actions.remove(2);
            }
            1 => {
                function.cleanup_plans[0].actions.push(raw::DropAction::DropPlace(raw::PlaceId(0)));
            }
            2 => {
                function.cleanup_plans[0].actions.swap(0, 2);
            }
            _ => {
                if let raw::Terminator::Return { cleanup, .. } =
                    &mut function.blocks[0].terminators[0].kind
                {
                    *cleanup = raw::CleanupPlanId(0);
                }
            }
        }
        fixture.rejects(raw, "ZRYNA-I3012");
    }
}

#[test]
fn indexed_borrow_noncopy_referents_cannot_escape_through_copy_read_or_write() {
    let fixture = Fixture::new(Container::Vec, Element::String);
    for write in [false, true] {
        let mut raw = fixture.seed(raw::BorrowAccess::Exclusive);
        fixture.verify(raw.clone());
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        let instruction = if write {
            raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::BorrowWrite {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(4),
                },
            }
        } else {
            function.places.push(raw::Place {
                id: raw::PlaceId(3),
                ty: fixture.element,
                span,
                kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
            });
            raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(5),
                    ty: fixture.element,
                    span,
                }),
                span,
                kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
            }
        };
        function.blocks[0].instructions.insert(1, instruction);
        fixture.rejects(raw, "ZRYNA-I3005");
    }
}

#[test]
fn indexed_borrow_lexical_authority_cannot_escape_exit_or_cfg_edge() {
    let fixture = Fixture::new(Container::Vec, Element::String);
    let seed = fixture.seed(raw::BorrowAccess::Shared);
    fixture.verify(seed.clone());
    for exit in 0..3 {
        let mut raw = seed.clone();
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        function.blocks[0].instructions.pop();
        match exit {
            0 => {}
            1 => {
                function.blocks[0].terminators[0].kind = raw::Terminator::Trap {
                    identity: raw::TrapIdentity::BoundsV1,
                    cleanup: raw::CleanupPlanId(1),
                };
            }
            _ => {
                let terminators = function.blocks[0].terminators.clone();
                function.blocks.push(raw::Block {
                    id: raw::BlockId(1),
                    parameters: vec![],
                    instructions: vec![end_borrow(0, span)],
                    terminators,
                });
                function.blocks[0].terminators[0].kind =
                    raw::Terminator::Jump(raw::Edge { target: raw::BlockId(1), arguments: vec![] });
            }
        }
        fixture.rejects(raw, "ZRYNA-I3011");
    }
}

#[test]
fn indexed_borrow_rejects_moved_uninitialized_or_partially_available_container() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::String);
        let seed = fixture.seed(raw::BorrowAccess::Shared);
        fixture.verify(seed.clone());
        for moved in [false, true] {
            let mut raw = seed.clone();
            let function = &mut raw.modules[0].functions[0];
            let span = function.span;
            function.places.push(raw::Place {
                id: raw::PlaceId(3),
                ty: fixture.root,
                span,
                kind: if moved {
                    raw::PlaceKind::Temporary(raw::ValueId(5))
                } else {
                    raw::PlaceKind::Local(0)
                },
            });
            if moved {
                function.blocks[0].instructions.insert(
                    0,
                    raw::Instruction {
                        result: Some(raw::ValueDefinition {
                            id: raw::ValueId(5),
                            ty: fixture.root,
                            span,
                        }),
                        span,
                        kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
                    },
                );
                for plan in &mut function.cleanup_plans {
                    *plan.actions.last_mut().expect("container") =
                        raw::DropAction::DropPlace(raw::PlaceId(3));
                }
            } else if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
                &mut function.blocks[0].instructions[0].kind
            {
                definition.place = raw::PlaceId(3);
            }
            fixture.rejects(raw, "ZRYNA-I3011");
        }
    }
    let fixture = Fixture::new(Container::Array, Element::String);
    let mut raw = fixture.seed(raw::BorrowAccess::Shared);
    fixture.verify(raw.clone());
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(3),
            ty: fixture.element,
            span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 0 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: fixture.element,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
    ]);
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.element, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(3) },
        },
    );
    for plan in &mut function.cleanup_plans {
        plan.actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(4)));
    }
    fixture.rejects(raw, "ZRYNA-I3011");
}
