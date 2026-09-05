use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn replacing_callee(fixture: &Fixture, span: zryna_source::Span, traps: bool) -> raw::Function {
    raw::Function {
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
                kind: if traps {
                    raw::Terminator::Trap {
                        identity: raw::TrapIdentity::BoundsV1,
                        cleanup: raw::CleanupPlanId(0),
                    }
                } else {
                    raw::Terminator::Return {
                        value: raw::ValueId(1),
                        cleanup: raw::CleanupPlanId(0),
                    }
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] }],
    }
}

fn refined_call(fixture: &Fixture, traps: bool) -> raw::Program {
    let mut raw = fixture.seed(raw::BorrowAccess::Exclusive);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.element,
        span,
        kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 0 },
    });
    let before = function.cleanup_plans[0].actions.clone();
    let after = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    function.cleanup_plans = [before.clone(), after.clone(), after, before]
        .into_iter()
        .enumerate()
        .map(|(id, actions)| raw::CleanupPlan {
            id: raw::CleanupPlanId(u32::try_from(id).expect("four plans")),
            span,
            actions,
        })
        .collect();
    let mut matched = function.blocks.remove(0);
    matched.id = raw::BlockId(1);
    matched.instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.integer, span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![
                    raw::CallArgument::Value(raw::ValueId(4)),
                    raw::CallArgument::Value(raw::ValueId(2)),
                    raw::CallArgument::Borrow(raw::BorrowId(0)),
                ],
                cleanup: raw::CleanupPlanId(1),
            },
        },
    );
    matched.terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(2) };
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::EnumMatch {
                    place: raw::PlaceId(3),
                    arms: (0..2)
                        .map(|variant| raw::EnumArm {
                            variant,
                            edge: raw::Edge {
                                target: raw::BlockId(variant + 1),
                                arguments: vec![],
                            },
                        })
                        .collect(),
                },
            }],
        },
        matched,
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(3),
                },
            }],
        },
    ];
    raw.modules[0].functions.push(replacing_callee(fixture, span, traps));
    raw
}

#[test]
fn indexed_borrow_call_trap_cleanup_forgets_pre_call_descendant_enum_variant() {
    let fixture = Fixture::new(Container::Array, Element::Enum);
    for traps in [false, true] {
        let raw = refined_call(&fixture, traps);
        for _ in 0..2 {
            let verified = fixture.verify(raw.clone());
            let function =
                verified.modules().next().expect("module").functions().next().expect("caller");
            let block = function.blocks().nth(1).expect("matched variant");
            let mut instructions = block.instructions();
            let begin = instructions.next().expect("indexed begin");
            let before = begin
                .derived_drop_actions()
                .find(|drop| drop.root().index() == 0)
                .expect("container before call");
            assert_eq!(
                before
                    .active_variants()
                    .find(|variant| variant.place().index() == 3)
                    .map(VerifiedActiveVariant::variant),
                Some(0)
            );
            let call = instructions.next().expect("call");
            let failed = call.derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(failed.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(), [1, 0]);
            let container = failed
                .iter()
                .find(|drop| drop.root().index() == 0)
                .expect("container on call trap");
            assert_eq!(
                container
                    .active_variants()
                    .find(|variant| variant.place().index() == 3)
                    .map(VerifiedActiveVariant::variant),
                None,
                "callee may replace the enum before trapping; cleanup must use its runtime tag"
            );
            assert_eq!(
                call.failure_ended_borrows()
                    .map(super::super::BorrowIdentity::index)
                    .collect::<Vec<_>>(),
                [0]
            );
            assert_eq!(failed, block.terminator().derived_drop_actions().collect::<Vec<_>>());
        }
    }
}
