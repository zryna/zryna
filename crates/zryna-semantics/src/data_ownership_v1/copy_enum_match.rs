use std::collections::BTreeMap;

use super::{
    Binding, Decl, Errors, RawDataDeclarationKind, RawExpressionKind, RawStatementKind,
    SemanticInput, Ty, TypeCategory, layout, raw, raw_layout, semantic_type, syntax,
};
use crate::data_ownership_v1::diagnostics::span;
use crate::data_ownership_v1::type_model::require_current_type_only_boundary;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn lower_enum_match_function<'a>(
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
    let root =
        usize::try_from(function.body.root_block).ok().and_then(|i| function.body.blocks.get(i))?;
    let [statement_id] = root.statements.as_slice() else {
        return None;
    };
    let statement =
        usize::try_from(*statement_id).ok().and_then(|i| function.body.statements.get(i))?;
    let RawStatementKind::Return { value: returned, .. } = statement.kind else {
        return None;
    };
    let match_expr =
        usize::try_from(returned).ok().and_then(|i| function.body.expressions.get(i))?;
    let RawExpressionKind::Match { scrutinee, arms, .. } = &match_expr.kind else {
        return None;
    };

    let mut parameters = Vec::with_capacity(function.parameters.len());
    let mut places = Vec::with_capacity(function.parameters.len() + arms.len());
    let mut bindings: BTreeMap<String, Binding> = BTreeMap::new();
    let mut next_value = 0_u32;
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            declarations,
            graph,
            node_types,
            errors,
        )?;
        require_current_type_only_boundary(
            ty,
            span(input.sources(), parameter.span),
            function.export_span.is_some(),
            errors,
        )?;
        parameters.push(raw::ValueDefinition {
            id: raw::ValueId(next_value),
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
        });
        next_value = next_value.checked_add(1)?;
        let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
        places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: span(input.sources(), parameter.span),
            kind: raw::PlaceKind::Parameter(u32::try_from(index).ok()?),
        });
        if bindings.keys().any(|name| name.eq_ignore_ascii_case(&parameter.name.text)) {
            errors.at(
                "ZRYNA-M3002",
                span(input.sources(), parameter.name.span),
                format!("parameter '{}' is declared more than once", parameter.name.text),
                "give every parameter one exact name",
            );
            return None;
        }
        bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    }
    let scrutinee_expr =
        usize::try_from(*scrutinee).ok().and_then(|i| function.body.expressions.get(i))?;
    let RawExpressionKind::Reference { name: scrutinee_name } = &scrutinee_expr.kind else {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "enum match scrutinee must be an addressable place",
            "match a parameter or initialized local enum place",
        );
        return None;
    };
    let scrutinee_binding = bindings.get(&scrutinee_name.text).cloned().or_else(|| {
        errors.at(
            "ZRYNA-M3002",
            span(input.sources(), scrutinee_name.span),
            format!("name '{}' is not declared", scrutinee_name.text),
            "match one declared enum place",
        );
        None
    })?;
    let record = layouts.type_by_id(scrutinee_binding.ty.layout)?;
    let nominal = record.nominal_identity().or_else(|| {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "match scrutinee is not a nominal enum",
            "match a value of one exact declared enum type",
        );
        None
    })?;
    if record.category() != TypeCategory::Enum {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), scrutinee_expr.span),
            "match scrutinee is not an enum",
            "match a value of one exact declared enum type",
        );
        return None;
    }
    let nominal_module = usize::try_from(nominal.0).ok()?;
    let nominal_declaration = usize::try_from(nominal.1).ok()?;
    let enum_decl = declarations
        .iter()
        .find(|d| d.module == nominal_module && d.declaration == nominal_declaration)?;
    let RawDataDeclarationKind::Enum { variants, .. } =
        &input.syntax().files()[enum_decl.module].data_declarations()[enum_decl.declaration].kind
    else {
        return None;
    };
    if arms.len() != variants.len() {
        errors.at(
            "ZRYNA-M3009",
            span(input.sources(), match_expr.span),
            format!(
                "enum match has {} arms but '{}' has {} variants",
                arms.len(),
                enum_decl.name,
                variants.len()
            ),
            "provide every variant exactly once and no extra arms",
        );
        return None;
    }

    let mut blocks = vec![raw::Block {
        id: raw::BlockId(0),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: Vec::new(),
    }];
    let mut enum_arms = Vec::with_capacity(arms.len());
    let mut cleanup_plans = Vec::with_capacity(arms.len());
    let mut seen = vec![false; variants.len()];
    let variant_ordinal = |arm: &syntax::RawMatchArm| {
        variants
            .iter()
            .position(|variant| variant.name.text == arm.variant.text)
            .unwrap_or(usize::MAX)
    };
    let mut ordered_arms = arms.iter().collect::<Vec<_>>();
    ordered_arms.sort_by_key(|arm| variant_ordinal(arm));
    for arm in ordered_arms {
        if arm.type_name.text != enum_decl.name {
            errors.at(
                "ZRYNA-M3009",
                span(input.sources(), arm.type_name.span),
                "match arm names a different enum",
                "use the scrutinee's exact enum name on every arm",
            );
            return None;
        }
        let (ordinal, variant) =
            variants.iter().enumerate().find(|(_, v)| v.name.text == arm.variant.text).or_else(
                || {
                    errors.at(
                        "ZRYNA-M3009",
                        span(input.sources(), arm.variant.span),
                        format!("enum '{}' has no variant '{}'", enum_decl.name, arm.variant.text),
                        "use every declared variant exactly once",
                    );
                    None
                },
            )?;
        if seen[ordinal] {
            errors.at(
                "ZRYNA-M3009",
                span(input.sources(), arm.variant.span),
                format!("variant '{}' appears more than once", arm.variant.text),
                "provide every variant exactly once",
            );
            return None;
        }
        seen[ordinal] = true;
        let block_id = raw::BlockId(u32::try_from(blocks.len()).ok()?);
        let mut arm_bindings = bindings.clone();
        match (variant.payload_type, &arm.binding) {
            (None, None) => {}
            (Some(payload_type), Some(binding)) => {
                let payload_ty = semantic_type(
                    file,
                    payload_type,
                    module,
                    declarations,
                    graph,
                    node_types,
                    errors,
                )?;
                let payload_place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                places.push(raw::Place {
                    id: payload_place,
                    ty: payload_ty.ir,
                    span: span(input.sources(), binding.span),
                    kind: raw::PlaceKind::EnumPayload {
                        base: scrutinee_binding.place,
                        variant: u32::try_from(ordinal).ok()?,
                    },
                });
                if arm_bindings.keys().any(|name| name.eq_ignore_ascii_case(&binding.text)) {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), binding.span),
                        format!(
                            "match binding '{}' collides under portable ASCII case folding",
                            binding.text
                        ),
                        "choose a binding distinct from parameters and locals",
                    );
                    return None;
                }
                arm_bindings.insert(
                    binding.text.clone(),
                    Binding { ty: payload_ty, place: payload_place, mutable: false },
                );
            }
            _ => {
                errors.at(
                    "ZRYNA-M3009",
                    span(input.sources(), arm.span),
                    "match payload binding does not match the declared variant",
                    "bind exactly one name only on a payload variant",
                );
                return None;
            }
        }
        let arm_expr =
            usize::try_from(arm.value).ok().and_then(|i| function.body.expressions.get(i))?;
        let arm_span = span(input.sources(), arm_expr.span);
        let (arm_ty, instruction_kind) = match &arm_expr.kind {
            RawExpressionKind::I32Literal { spelling } => {
                let value = spelling.parse::<i32>().ok().or_else(|| {
                    errors.at(
                        "ZRYNA-M3008",
                        arm_span,
                        "match-arm integer literal is outside i32",
                        "use an i32 literal",
                    );
                    None
                })?;
                let ty = node_types.iter().flatten().find(|ty| ty.category == TypeCategory::I32)?;
                let ty = *ty;
                (ty, raw::InstructionKind::I32Literal(value))
            }
            RawExpressionKind::BoolLiteral { value } => {
                let ty =
                    node_types.iter().flatten().find(|ty| ty.category == TypeCategory::Bool)?;
                let ty = *ty;
                (ty, raw::InstructionKind::BoolLiteral(*value))
            }
            RawExpressionKind::Reference { name } => {
                let binding = arm_bindings.get(&name.text).cloned().or_else(|| {
                    errors.at(
                        "ZRYNA-M3002",
                        span(input.sources(), name.span),
                        format!("match-arm name '{}' is not declared", name.text),
                        "reference the payload binding or a parameter",
                    );
                    None
                })?;
                (binding.ty, raw::InstructionKind::CopyFromPlace { place: binding.place })
            }
            _ => {
                errors.at(
                    "ZRYNA-M3009",
                    arm_span,
                    "nested aggregate operations in match arms are outside the M3 match oracle",
                    "return a scalar literal, parameter, or payload binding from each arm",
                );
                return None;
            }
        };
        if arm_ty.layout != result.layout {
            errors.at(
                "ZRYNA-M3009",
                arm_span,
                "enum match arms do not all have the declared result type",
                "make every arm produce one exact common type",
            );
            return None;
        }
        let value_id = raw::ValueId(next_value);
        next_value = next_value.checked_add(1)?;
        let definition = raw::ValueDefinition { id: value_id, ty: arm_ty.ir, span: arm_span };
        blocks.push(raw::Block {
            id: block_id,
            parameters: Vec::new(),
            instructions: vec![raw::Instruction {
                result: Some(definition),
                span: arm_span,
                kind: instruction_kind,
            }],
            terminators: Vec::new(),
        });
        enum_arms.push(raw::EnumArm {
            variant: u32::try_from(ordinal).ok()?,
            edge: raw::Edge { target: block_id, arguments: Vec::new() },
        });
        let arm_block = blocks.last_mut().expect("just pushed match arm block");
        let cleanup = raw::CleanupPlanId(u32::try_from(cleanup_plans.len()).ok()?);
        cleanup_plans.push(raw::CleanupPlan {
            id: cleanup,
            span: span(input.sources(), arm.span),
            actions: Vec::new(),
        });
        arm_block.terminators.push(raw::SpannedTerminator {
            span: span(input.sources(), arm.span),
            kind: raw::Terminator::Return { value: value_id, cleanup },
        });
    }
    blocks[0].terminators.push(raw::SpannedTerminator {
        span: span(input.sources(), match_expr.span),
        kind: raw::Terminator::EnumMatch { place: scrutinee_binding.place, arms: enum_arms },
    });
    if function.export_span.is_some() {
        errors.at(
            "ZRYNA-M3010",
            span(input.sources(), function.span),
            "public aggregate signatures are outside scalar ABI v1",
            "keep enum-match functions internal and export only bool/i32 signatures",
        );
        return None;
    }
    Some(raw::Function {
        id: raw::FunctionId {
            module: raw::ModuleId(u32::try_from(module).ok()?),
            declaration: u32::try_from(declaration).ok()?,
        },
        entry_export: None,
        span: span(input.sources(), function.span),
        parameters,
        borrow_parameters: Vec::new(),
        result: result.ir,
        places,
        blocks,
        cleanup_plans,
    })
}
