use std::collections::BTreeSet;

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_syntax::v4::{
    self as syntax, RawDataDeclarationKind, RawExpressionKind, RawStatementKind,
};

use super::SemanticInput;
use super::diagnostics::Errors;
use super::layout_graph::{Decl, semantic_type};
use super::owned_aggregate_lowering::{
    aggregate_graph_is_supported, complete_owned_projection_shape,
};
use super::owned_control_flow_resources::enum_payload_move_resource_violation;
use super::type_model::{OwnedStaticProjectionKind, Ty};
use crate::data_ownership_v1::diagnostics::span;

pub(super) fn is_private_owned_enum_payload_move_candidate(
    function: &syntax::RawFunctionSyntax,
) -> bool {
    let Some(root) = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))
    else {
        return false;
    };
    let Some(statement) = root
        .statements
        .first()
        .and_then(|id| usize::try_from(*id).ok())
        .and_then(|index| function.body.statements.get(index))
    else {
        return false;
    };
    let RawStatementKind::LocalDeclaration { initializer, .. } = statement.kind else {
        return false;
    };
    usize::try_from(initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))
        .is_some_and(|expression| matches!(expression.kind, RawExpressionKind::Match { .. }))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn lower_private_owned_enum_payload_move_function<'a>(
    input: SemanticInput<'a>,
    module: usize,
    declaration: usize,
    function: &syntax::RawFunctionSyntax,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    result: Ty,
    errors: &mut Errors<'a>,
) -> Option<raw::Function> {
    let file = &input.syntax().files()[module];
    let function_span = span(input.sources(), function.span);
    let root = usize::try_from(function.body.root_block)
        .ok()
        .and_then(|index| function.body.blocks.get(index))?;
    let [local_id, return_id] = root.statements.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            function_span,
            "owned enum payload extraction requires one local initializer and one final return",
            "bind the one-arm match result to one exact local, then return that local",
        );
        return None;
    };
    let local_statement =
        usize::try_from(*local_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::LocalDeclaration {
        mutable,
        name: local_name,
        type_syntax,
        initializer,
        ..
    } = &local_statement.kind
    else {
        return None;
    };
    if *mutable {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), local_statement.span),
            "owned enum payload extraction requires an immutable direct local",
            "declare the exact match result with const",
        );
        return None;
    }
    let local_ty =
        semantic_type(file, *type_syntax, module, declarations, graph, node_types, errors)?;
    if local_ty != result
        || result.is_copy()
        || !matches!(result.category, TypeCategory::Struct | TypeCategory::FixedArray)
        || !aggregate_graph_is_supported(result, layouts, &mut BTreeSet::new())
    {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), local_statement.span),
            "enum payload result is outside the exact owned Struct/fixed-array extraction slice",
            "use one exact acyclic non-Copy Struct or fixed array with bool, i32, and String leaves",
        );
        return None;
    }
    let return_statement =
        usize::try_from(*return_id).ok().and_then(|index| function.body.statements.get(index))?;
    let RawStatementKind::Return { value: returned, .. } = return_statement.kind else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), return_statement.span),
            "owned enum payload extraction must end with the direct local return",
            "return the exact initialized payload local as the final statement",
        );
        return None;
    };
    let returned_expression =
        usize::try_from(returned).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &returned_expression.kind,
        RawExpressionKind::Reference { name } if name.text == local_name.text
    ) {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), returned_expression.span),
            "owned enum payload extraction continuation must return its exact local",
            "return the match-initialized local without another expression",
        );
        return None;
    }
    let match_expression = usize::try_from(*initializer)
        .ok()
        .and_then(|index| function.body.expressions.get(index))?;
    let RawExpressionKind::Match { scrutinee, arms, .. } = &match_expression.kind else {
        return None;
    };
    let [parameter] = function.parameters.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            function_span,
            "owned enum payload extraction requires one exact enum source parameter",
            "pass one single-variant owned enum source",
        );
        return None;
    };
    if parameter.name.text.eq_ignore_ascii_case(&local_name.text) {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), local_name.span),
            format!("local '{}' collides under portable ASCII case folding", local_name.text),
            "give the result local a name distinct from the source parameter",
        );
        return None;
    }
    let source_ty = semantic_type(
        file,
        parameter.type_syntax,
        module,
        declarations,
        graph,
        node_types,
        errors,
    )?;
    let source_record = layouts.type_by_id(source_ty.layout)?;
    let Some((nominal_module, nominal_declaration)) = source_record.nominal_identity() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), parameter.span),
            "owned payload source is not one exact nominal enum",
            "use a declared enum with exactly one payload variant",
        );
        return None;
    };
    if source_record.category() != TypeCategory::Enum
        || source_ty.is_copy()
        || usize::try_from(nominal_module).ok() != Some(module)
    {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), parameter.span),
            "owned payload source is outside the private single-variant enum slice",
            "use one private same-module non-Copy enum source",
        );
        return None;
    }
    let enum_decl = declarations.iter().find(|decl| {
        decl.module == module && u32::try_from(decl.declaration).ok() == Some(nominal_declaration)
    })?;
    let RawDataDeclarationKind::Enum { variants, .. } =
        &file.data_declarations()[enum_decl.declaration].kind
    else {
        return None;
    };
    let [variant] = variants.as_slice() else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), file.data_declarations()[enum_decl.declaration].span),
            "owned payload extraction requires an enum with exactly one variant",
            "use one enum containing exactly one payload-bearing variant",
        );
        return None;
    };
    let Some(payload_type) = variant.payload_type else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), variant.span),
            "owned payload extraction requires a payload-bearing variant",
            "give the enum's only variant the exact result payload type",
        );
        return None;
    };
    let payload_ty =
        semantic_type(file, payload_type, module, declarations, graph, node_types, errors)?;
    if payload_ty != result {
        errors.at(
            "ZRYNA-M3007",
            span(input.sources(), variant.span),
            "enum payload type does not match the exact extracted local type",
            "use the variant's exact payload type for the local and function result",
        );
        return None;
    }
    let scrutinee_expression =
        usize::try_from(*scrutinee).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &scrutinee_expression.kind,
        RawExpressionKind::Reference { name } if name.text == parameter.name.text
    ) {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expression.span),
            "owned payload match must refine the exact source parameter",
            "match the one declared enum source directly",
        );
        return None;
    }
    let [arm] = arms.as_slice() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), match_expression.span),
            "owned payload match requires exactly one exhaustive arm",
            "provide the enum's only variant exactly once",
        );
        return None;
    };
    let Some(binding) = arm.binding.as_ref() else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm.span),
            "owned payload arm must bind its payload",
            "bind one name and return that exact binding from the arm",
        );
        return None;
    };
    if arm.type_name.text != enum_decl.name || arm.variant.text != variant.name.text {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm.span),
            "owned payload arm does not name the source enum's only variant",
            "match the exact enum and variant spelling",
        );
        return None;
    }
    if binding.text.eq_ignore_ascii_case(&parameter.name.text)
        || binding.text.eq_ignore_ascii_case(&local_name.text)
    {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), binding.span),
            format!("match binding '{}' collides under portable ASCII case folding", binding.text),
            "give the payload binding a distinct portable name",
        );
        return None;
    }
    let arm_expression =
        usize::try_from(arm.value).ok().and_then(|index| function.body.expressions.get(index))?;
    if !matches!(
        &arm_expression.kind,
        RawExpressionKind::Reference { name } if name.text == binding.text
    ) {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), arm_expression.span),
            "owned payload arm must yield its exact bound payload",
            "return the payload binding directly from the one match arm",
        );
        return None;
    }

    let Some(shape) = complete_owned_projection_shape(payload_ty, layouts) else {
        errors.at(
            "ZRYNA-M3016",
            span(input.sources(), variant.span),
            "enum payload topology is outside the bounded owned aggregate slice",
            "use one acyclic Struct or fixed array with bool, i32, and String leaves",
        );
        return None;
    };
    if enum_payload_move_resource_violation(shape.len()) {
        errors.at(
            "ZRYNA-M3201",
            span(input.sources(), match_expression.span),
            "derived enum payload extraction exceeds an M3 function resource limit",
            "reduce the payload's static Struct/fixed-array topology",
        );
        return None;
    }

    let parameter_span = span(input.sources(), parameter.span);
    let arm_span = span(input.sources(), arm.span);
    let local_span = span(input.sources(), local_statement.span);
    let return_span = span(input.sources(), return_statement.span);
    let source_place = raw::PlaceId(0);
    let payload_place = raw::PlaceId(1);
    let mut places = vec![
        raw::Place {
            id: source_place,
            ty: source_ty.ir,
            span: parameter_span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: payload_place,
            ty: payload_ty.ir,
            span: span(input.sources(), binding.span),
            kind: raw::PlaceKind::EnumPayload { base: source_place, variant: 0 },
        },
    ];
    let mut descendants = Vec::<raw::PlaceId>::with_capacity(shape.len());
    for entry in &shape {
        let base = entry.parent.map_or(payload_place, |index| descendants[index]);
        let kind = match entry.kind {
            OwnedStaticProjectionKind::StructField { ordinal } => {
                raw::PlaceKind::StructField { base, ordinal }
            }
            OwnedStaticProjectionKind::FixedArrayConstant { index } => {
                raw::PlaceKind::FixedArrayConstant { base, index }
            }
        };
        let id = raw::PlaceId(u32::try_from(places.len()).ok()?);
        places.push(raw::Place { id, ty: entry.ty.ir, span: arm_span, kind });
        descendants.push(id);
    }
    let moved_value = raw::ValueId(1);
    let moved_owner = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: moved_owner,
        ty: payload_ty.ir,
        span: arm_span,
        kind: raw::PlaceKind::Temporary(moved_value),
    });
    let local_place = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: local_place,
        ty: payload_ty.ir,
        span: local_span,
        kind: raw::PlaceKind::Local(0),
    });
    let returned_value = raw::ValueId(2);
    let returned_owner = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: returned_owner,
        ty: payload_ty.ir,
        span: return_span,
        kind: raw::PlaceKind::Temporary(returned_value),
    });

    let cleanup = raw::CleanupPlanId(0);
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: function_span,
        parameters: vec![raw::ValueDefinition {
            id: raw::ValueId(0),
            ty: source_ty.ir,
            span: parameter_span,
        }],
        borrow_parameters: Vec::new(),
        result: result.ir,
        places,
        blocks: vec![
            raw::Block {
                id: raw::BlockId(0),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminators: vec![raw::SpannedTerminator {
                    span: span(input.sources(), match_expression.span),
                    kind: raw::Terminator::EnumMatch {
                        place: source_place,
                        arms: vec![raw::EnumArm {
                            variant: 0,
                            edge: raw::Edge { target: raw::BlockId(1), arguments: Vec::new() },
                        }],
                    },
                }],
            },
            raw::Block {
                id: raw::BlockId(1),
                parameters: Vec::new(),
                instructions: vec![
                    raw::Instruction {
                        result: Some(raw::ValueDefinition {
                            id: moved_value,
                            ty: payload_ty.ir,
                            span: arm_span,
                        }),
                        span: arm_span,
                        kind: raw::InstructionKind::MoveFromPlace { place: payload_place },
                    },
                    raw::Instruction {
                        result: None,
                        span: local_span,
                        kind: raw::InstructionKind::InitializePlace {
                            place: local_place,
                            value: moved_value,
                        },
                    },
                    raw::Instruction {
                        result: None,
                        span: arm_span,
                        kind: raw::InstructionKind::DropPlace { place: source_place },
                    },
                ],
                terminators: vec![raw::SpannedTerminator {
                    span: arm_span,
                    kind: raw::Terminator::Jump(raw::Edge {
                        target: raw::BlockId(2),
                        arguments: Vec::new(),
                    }),
                }],
            },
            raw::Block {
                id: raw::BlockId(2),
                parameters: Vec::new(),
                instructions: vec![raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: returned_value,
                        ty: payload_ty.ir,
                        span: return_span,
                    }),
                    span: return_span,
                    kind: raw::InstructionKind::MoveFromPlace { place: local_place },
                }],
                terminators: vec![raw::SpannedTerminator {
                    span: return_span,
                    kind: raw::Terminator::Return { value: returned_value, cleanup },
                }],
            },
        ],
        cleanup_plans: vec![raw::CleanupPlan {
            id: cleanup,
            span: return_span,
            actions: Vec::new(),
        }],
    })
}
