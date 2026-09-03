use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, raw as raw_layout};
use zryna_syntax::v4 as syntax;

use super::diagnostics::Errors;
use super::function_catalog::FunctionCatalog;
use super::layout_graph::Decl;
use super::root_borrow_execution::{emit_root_borrow_arm, emit_root_borrow_initializer};
use super::root_borrow_shape_planning::plan_private_root_borrow_function;
use super::type_model::{RootBorrowPlan, RootBorrowShape, RootBorrowStep, Ty};
use super::{SemanticInput, span};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn lower_private_root_borrow_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &'a syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    catalog: &'a FunctionCatalog,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let plan = plan_private_root_borrow_function(
        input,
        module,
        function,
        declarations,
        graph,
        node_types,
        layouts,
        catalog,
        result,
        errors,
    )?;
    let RootBorrowPlan {
        root_ty,
        root_initializer,
        root_at,
        shape,
        aliases,
        reads,
        writes,
        calls,
        call_values,
        return_at,
    } = plan;
    let mut values = 0_u32;
    let root_place = raw::PlaceId(0);
    let instruction_capacity = aliases
        .checked_mul(2)?
        .checked_add(reads.checked_mul(2)?)?
        .checked_add(writes.checked_mul(2)?)?
        .checked_add(call_values)?
        .checked_add(calls.checked_mul(2)?)?
        .checked_add(4)?;
    let mut root_initialization = Vec::new();
    let root_value =
        emit_root_borrow_initializer(root_initializer, &mut values, &mut root_initialization)?;
    root_initialization.push(raw::Instruction {
        result: None,
        span: root_at,
        kind: raw::InstructionKind::InitializePlace { place: root_place, value: root_value },
    });
    let mut places = Vec::with_capacity(reads.saturating_add(calls).saturating_add(1));
    places.push(raw::Place {
        id: root_place,
        ty: root_ty.ir,
        span: root_at,
        kind: raw::PlaceKind::Local(0),
    });
    let cleanup = raw::CleanupPlanId(0);
    let call_at = match &shape {
        RootBorrowShape::Straight(arm) => arm.steps.iter().find_map(|step| match step {
            RootBorrowStep::Call(call) => Some(call.at),
            _ => None,
        }),
        RootBorrowShape::Loop { .. } | RootBorrowShape::Conditional { .. } => None,
    };
    debug_assert_eq!(call_at.is_some(), calls == 1);
    let blocks = match shape {
        RootBorrowShape::Straight(arm) => {
            let mut instructions = Vec::with_capacity(instruction_capacity);
            instructions.append(&mut root_initialization);
            let mut call_cleanup = (calls == 1).then_some(raw::CleanupPlanId(1));
            emit_root_borrow_arm(
                arm,
                root_place,
                true,
                &mut values,
                &mut places,
                &mut instructions,
                &mut call_cleanup,
            )?;
            debug_assert!(call_cleanup.is_none());
            let returned = raw::ValueId(values);
            instructions.push(raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            });
            vec![raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions,
                terminators: vec![raw::SpannedTerminator {
                    span: return_at,
                    kind: raw::Terminator::Return { value: returned, cleanup },
                }],
            }]
        }
        RootBorrowShape::Conditional { condition_at, branch_at, then_arm, else_arm } => {
            let mut entry = Vec::with_capacity(3);
            entry.append(&mut root_initialization);
            let condition = raw::ValueId(values);
            values = values.checked_add(1)?;
            entry.push(raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: condition,
                    ty: root_ty.ir,
                    span: condition_at,
                }),
                span: condition_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            });
            let mut then_instructions = Vec::with_capacity(instruction_capacity);
            let mut no_call_cleanup = None;
            emit_root_borrow_arm(
                then_arm,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut then_instructions,
                &mut no_call_cleanup,
            )?;
            let mut else_instructions = Vec::with_capacity(instruction_capacity);
            emit_root_borrow_arm(
                else_arm,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut else_instructions,
                &mut no_call_cleanup,
            )?;
            let returned = raw::ValueId(values);
            let join_instructions = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: entry,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: raw::BlockId(1), arguments: Vec::new() },
                            when_false: raw::Edge {
                                target: raw::BlockId(2),
                                arguments: Vec::new(),
                            },
                        },
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: Vec::new(),
                    instructions: then_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(3),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: Vec::new(),
                    instructions: else_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: branch_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(3),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(3),
                    parameters: Vec::new(),
                    instructions: join_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: return_at,
                        kind: raw::Terminator::Return { value: returned, cleanup },
                    }],
                },
            ]
        }
        RootBorrowShape::Loop { condition_at, loop_at, body } => {
            let entry = root_initialization;
            let condition = raw::ValueId(values);
            values = values.checked_add(1)?;
            let header = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: condition,
                    ty: root_ty.ir,
                    span: condition_at,
                }),
                span: condition_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            let mut body_instructions = Vec::with_capacity(instruction_capacity);
            let mut no_call_cleanup = None;
            emit_root_borrow_arm(
                body,
                root_place,
                false,
                &mut values,
                &mut places,
                &mut body_instructions,
                &mut no_call_cleanup,
            )?;
            let returned = raw::ValueId(values);
            let exit = vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: returned,
                    ty: root_ty.ir,
                    span: return_at,
                }),
                span: return_at,
                kind: raw::InstructionKind::CopyFromPlace { place: root_place },
            }];
            vec![
                raw::Block {
                    id: raw::BlockId(0),
                    parameters: Vec::new(),
                    instructions: entry,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(1),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(1),
                    parameters: Vec::new(),
                    instructions: header,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Branch {
                            condition,
                            when_true: raw::Edge { target: raw::BlockId(2), arguments: Vec::new() },
                            when_false: raw::Edge {
                                target: raw::BlockId(3),
                                arguments: Vec::new(),
                            },
                        },
                    }],
                },
                raw::Block {
                    id: raw::BlockId(2),
                    parameters: Vec::new(),
                    instructions: body_instructions,
                    terminators: vec![raw::SpannedTerminator {
                        span: loop_at,
                        kind: raw::Terminator::Jump(raw::Edge {
                            target: raw::BlockId(1),
                            arguments: Vec::new(),
                        }),
                    }],
                },
                raw::Block {
                    id: raw::BlockId(3),
                    parameters: Vec::new(),
                    instructions: exit,
                    terminators: vec![raw::SpannedTerminator {
                        span: return_at,
                        kind: raw::Terminator::Return { value: returned, cleanup },
                    }],
                },
            ]
        }
    };
    let _ = (aliases, reads, writes, call_values);
    let mut cleanup_plans = vec![raw::CleanupPlan {
        id: cleanup,
        span: span(input.sources(), function.body.span),
        actions: Vec::new(),
    }];
    if calls == 1 {
        cleanup_plans.push(raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span: call_at?,
            actions: Vec::new(),
        });
    }
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters: Vec::new(),
        borrow_parameters: Vec::new(),
        result: root_ty.ir,
        places,
        blocks,
        cleanup_plans,
    })
}
