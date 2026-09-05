use super::indexed_borrow_fixture::{Container, ELEMENTS, Element, Fixture};
use super::*;

fn replacement(fixture: &Fixture) -> raw::Program {
    let mut raw = fixture.seed(raw::BorrowAccess::Exclusive);
    let function = &mut raw.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: None,
            span: function.span,
            kind: raw::InstructionKind::BorrowReplace {
                borrow: raw::BorrowId(0),
                value: raw::ValueId(4),
            },
        },
    );
    function.cleanup_plans[1]
        .actions
        .retain(|action| *action != raw::DropAction::DropPlace(raw::PlaceId(2)));
    raw
}

#[test]
fn indexed_borrow_owned_replacement_seals_old_referent_drop_and_retains_complete_container() {
    for container in [Container::Array, Container::Vec] {
        for element in ELEMENTS {
            let fixture = Fixture::new(container, element);
            if fixture.is_copy(fixture.element) {
                continue;
            }
            let seed = replacement(&fixture);
            for _ in 0..2 {
                let verified = fixture.verify(seed.clone());
                let function = verified
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function");
                let block = function.blocks().next().expect("block");
                let commit = block.instructions().nth(1).expect("replacement");
                let authority = commit.borrow_replacement().expect("owned replacement authority");
                assert_eq!(authority.borrow().index(), 0);
                assert_eq!(authority.referent().index(), fixture.element.0);
                assert_eq!(authority.value().index(), 4);
                assert_eq!(authority.old_value_drop().borrow().index(), 0);
                assert_eq!(authority.old_value_drop().referent().index(), fixture.element.0);
                assert_eq!(
                    commit.derived_drop_actions().count(),
                    0,
                    "old element is not a whole-container drop"
                );
                assert!(commit.result().is_none());
                let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
                assert_eq!(
                    cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
                    [1, 0]
                );
                assert!(cleanup.iter().all(|action| action.moved_projections().count() == 0));
            }
        }
    }
}

#[test]
fn indexed_borrow_owned_replacement_rejects_mode_type_reuse_and_borrowed_rhs() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::Struct);
        let seed = replacement(&fixture);
        fixture.verify(seed.clone());
        let mut shared = seed.clone();
        if let raw::InstructionKind::BeginIndexedBorrow { definition, .. } =
            &mut shared.modules[0].functions[0].blocks[0].instructions[0].kind
        {
            definition.access = raw::BorrowAccess::Shared;
        }
        fixture.rejects(shared, "ZRYNA-I3005");
        let mut wrong = seed.clone();
        if let raw::InstructionKind::BorrowReplace { value, .. } =
            &mut wrong.modules[0].functions[0].blocks[0].instructions[1].kind
        {
            *value = raw::ValueId(2);
        }
        fixture.rejects(wrong, "ZRYNA-I3005");
        let mut reused = seed.clone();
        let instruction = reused.modules[0].functions[0].blocks[0].instructions[1].clone();
        reused.modules[0].functions[0].blocks[0].instructions.insert(2, instruction);
        fixture.rejects(reused, "ZRYNA-I3010");
        let mut borrowed_rhs = seed.clone();
        let function = &mut borrowed_rhs.modules[0].functions[0];
        let span = function.span;
        function.places.push(raw::Place {
            id: raw::PlaceId(3),
            ty: fixture.integer,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 1 },
        });
        function.blocks[0]
            .instructions
            .insert(1, begin_borrow(1, 3, raw::BorrowAccess::Shared, span));
        function.blocks[0].instructions.insert(3, end_borrow(1, span));
        fixture.rejects(borrowed_rhs, "ZRYNA-I3010");
        let mut inactive = seed;
        inactive.modules[0].functions[0].blocks[0].instructions.swap(1, 2);
        fixture.rejects(inactive, "ZRYNA-I3011");
    }
}

fn call_replacement(fixture: &Fixture) -> raw::Program {
    let mut raw = fixture.seed(raw::BorrowAccess::Exclusive);
    let span = raw.modules[0].functions[0].span;
    let callee = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![
            raw::ValueDefinition { id: raw::ValueId(0), ty: fixture.element, span },
            raw::ValueDefinition { id: raw::ValueId(1), ty: fixture.integer, span },
        ],
        borrow_parameters: vec![raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent: fixture.element,
            access: raw::BorrowAccess::Exclusive,
            span,
        }],
        result: fixture.integer,
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: fixture.element,
            span,
            kind: raw::PlaceKind::Parameter(0),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::BorrowReplace {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(0),
                },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] }],
    };
    let function = &mut raw.modules[0].functions[0];
    let remaining = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    function.cleanup_plans[1].actions = remaining.clone();
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(2),
        span,
        actions: remaining,
    });
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.integer, span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: callee.id,
                arguments: vec![
                    raw::CallArgument::Value(raw::ValueId(4)),
                    raw::CallArgument::Value(raw::ValueId(2)),
                    raw::CallArgument::Borrow(raw::BorrowId(0)),
                ],
                cleanup: raw::CleanupPlanId(1),
            },
        },
    );
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(2) };
    raw.modules[0].functions.push(callee);
    raw
}

#[test]
fn indexed_borrow_owned_call_replacement_transfers_rhs_but_preserves_caller_container() {
    for container in [Container::Array, Container::Vec] {
        for element in [
            Element::String,
            Element::Struct,
            Element::Enum,
            Element::Vec,
            Element::Shared,
            Element::Weak,
        ] {
            let fixture = Fixture::new(container, element);
            let seed = call_replacement(&fixture);
            let verified = fixture.verify(seed.clone());
            let module = verified.modules().next().expect("module");
            let functions = module.functions().collect::<Vec<_>>();
            let call = functions[0]
                .blocks()
                .next()
                .expect("caller block")
                .instructions()
                .nth(1)
                .expect("call");
            assert_eq!(
                call.failure_ended_borrows()
                    .map(super::super::BorrowIdentity::index)
                    .collect::<Vec<_>>(),
                [0]
            );
            assert_eq!(
                call.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
                [1, 0]
            );
            let commit = functions[1]
                .blocks()
                .next()
                .expect("callee block")
                .instructions()
                .next()
                .expect("commit");
            assert_eq!(commit.failure_ended_borrows().count(), 0);
            assert_eq!(
                commit.borrow_replacement().expect("parameter authority").referent().index(),
                fixture.element.0
            );
            let mut wrong = seed;
            wrong.modules[0].functions[1].borrow_parameters[0].referent = fixture.root;
            fixture.rejects(wrong, "ZRYNA-I3005");
        }
    }
}

#[test]
fn indexed_borrow_owned_call_rejects_consumption_with_live_rhs_descendant_borrow() {
    let fixture = Fixture::new(Container::Vec, Element::Struct);
    let seed = call_replacement(&fixture);
    fixture.verify(seed.clone());
    let mut raw = seed;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.integer,
        span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 1 },
    });
    function.blocks[0].instructions.insert(1, begin_borrow(1, 3, raw::BorrowAccess::Shared, span));
    function.blocks[0].instructions.insert(3, end_borrow(1, span));
    fixture.rejects(raw, "ZRYNA-I3010");
}

#[test]
fn indexed_borrow_replacement_cannot_launder_a_borrowed_rhs_through_local_storage() {
    for container in [Container::Array, Container::Vec] {
        let fixture = Fixture::new(container, Element::Struct);
        let mut raw = replacement(&fixture);
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        function.places.extend([
            raw::Place {
                id: raw::PlaceId(3),
                ty: fixture.element,
                span,
                kind: raw::PlaceKind::Local(0),
            },
            raw::Place {
                id: raw::PlaceId(4),
                ty: fixture.element,
                span,
                kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
            },
            raw::Place {
                id: raw::PlaceId(5),
                ty: fixture.integer,
                span,
                kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 1 },
            },
        ]);
        function.blocks[0].instructions.insert(
            1,
            raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::InitializePlace {
                    place: raw::PlaceId(3),
                    value: raw::ValueId(4),
                },
            },
        );
        function.blocks[0].instructions.insert(
            2,
            raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(5),
                    ty: fixture.element,
                    span,
                }),
                span,
                kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(3) },
            },
        );
        function.blocks[0].instructions[3].kind = raw::InstructionKind::BorrowReplace {
            borrow: raw::BorrowId(0),
            value: raw::ValueId(5),
        };
        fixture.verify(raw.clone());
        let function = &mut raw.modules[0].functions[0];
        function.blocks[0]
            .instructions
            .insert(1, begin_borrow(1, 5, raw::BorrowAccess::Shared, span));
        function.blocks[0].instructions.insert(5, end_borrow(1, span));
        fixture.rejects(raw, "ZRYNA-I3010");
    }
}
